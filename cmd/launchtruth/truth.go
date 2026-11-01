package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// recorder is a loopback npm registry and PyPI simple index that answers 404 to
// everything and remembers which package names were asked for. A 404 makes the
// launcher stop before anything is installed or run.
type recorder struct {
	mu    sync.Mutex
	paths []string
}

func (r *recorder) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	r.mu.Lock()
	r.paths = append(r.paths, req.URL.EscapedPath())
	r.mu.Unlock()
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusNotFound)
	fmt.Fprint(w, `{"error":"not found"}`)
}

func (r *recorder) take() []string {
	r.mu.Lock()
	defer r.mu.Unlock()
	p := r.paths
	r.paths = nil
	return p
}

// packageFromPath maps a registry request path to the package it asks for.
func packageFromPath(p string) (Package, bool) {
	if rest, ok := strings.CutPrefix(p, "/simple/"); ok {
		name := strings.Trim(rest, "/")
		if name == "" || strings.Contains(name, "/") {
			return Package{}, false
		}
		return Package{Ecosystem: "PyPI", Name: name}, true
	}
	raw := strings.TrimPrefix(p, "/")
	name, err := url.PathUnescape(raw)
	if err != nil || name == "" || strings.HasPrefix(name, "-/") {
		return Package{}, false
	}
	if strings.Count(name, "/") > 1 || (strings.Contains(name, "/") && !strings.HasPrefix(name, "@")) {
		return Package{}, false
	}
	return Package{Ecosystem: "npm", Name: name}, true
}

func launcherEnv(registry, home string) []string {
	return []string{
		"PATH=" + os.Getenv("PATH"),
		"HOME=" + home,
		"XDG_CACHE_HOME=" + filepath.Join(home, "cache"),
		"XDG_DATA_HOME=" + filepath.Join(home, "data"),
		"XDG_CONFIG_HOME=" + filepath.Join(home, "config"),
		"NPM_CONFIG_REGISTRY=" + registry + "/",
		"npm_config_cache=" + filepath.Join(home, "npm-cache"),
		"npm_config_update_notifier=false",
		"npm_config_fund=false",
		"npm_config_audit=false",
		"UV_INDEX_URL=" + registry + "/simple/",
		"UV_CACHE_DIR=" + filepath.Join(home, "uv-cache"),
		"UV_TOOL_DIR=" + filepath.Join(home, "uv-tools"),
		"UV_NO_CONFIG=1",
		"UV_PYTHON_DOWNLOADS=never",
	}
}

func measureTruth(cases []Case) ([]Truth, error) {
	rec := &recorder{}
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return nil, err
	}
	srv := &http.Server{Handler: rec}
	go srv.Serve(ln)
	defer srv.Close()
	registry := "http://" + ln.Addr().String()

	var out []Truth
	for _, c := range cases {
		home, err := os.MkdirTemp("", "launchtruth-")
		if err != nil {
			return nil, err
		}
		ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
		cmd := exec.CommandContext(ctx, c.Command, c.Args...)
		cmd.Dir = home
		cmd.Env = launcherEnv(registry, home)
		cmd.Stdin = nil
		exit := 0
		if out, err := cmd.CombinedOutput(); err != nil {
			var ee *exec.ExitError
			if !errors.As(err, &ee) {
				cancel()
				return nil, fmt.Errorf("%s: cannot start %s: %v", c.ID, c.Command, err)
			}
			exit = ee.ExitCode()
			if ctx.Err() != nil {
				cancel()
				return nil, fmt.Errorf("%s: %s timed out: %s", c.ID, c.Command, out)
			}
		}
		cancel()
		os.RemoveAll(home)

		t := Truth{ID: c.ID, Exit: exit, Requested: []Package{}}
		for _, p := range rec.take() {
			if pkg, ok := packageFromPath(p); ok {
				t.Requested = append(t.Requested, pkg)
			} else {
				t.Other = append(t.Other, p)
			}
		}
		t.Requested = normalizeAll(t.Requested)
		out = append(out, t)
	}
	return out, nil
}
