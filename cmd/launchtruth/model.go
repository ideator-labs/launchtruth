package main

import (
	"regexp"
	"sort"
	"strings"
)

// Package is one registry package: what a launcher fetches or a scanner checks.
type Package struct {
	Ecosystem string `json:"ecosystem"`
	Name      string `json:"name"`
}

// Case is one MCP server launch command plus the packages the runner docs say it installs.
type Case struct {
	ID       string    `json:"id"`
	Kind     string    `json:"kind"`
	Command  string    `json:"command"`
	Args     []string  `json:"args"`
	Installs []Package `json:"installs"`
	Note     string    `json:"note"`
	Source   string    `json:"source,omitempty"`
}

// Truth is what the real launcher asked the registry for when it ran the case.
type Truth struct {
	ID        string    `json:"id"`
	Requested []Package `json:"requested"`
	Other     []string  `json:"other_requests,omitempty"`
	Exit      int       `json:"launcher_exit"`
}

// ScanLine is one scanner's answer for one case: the package it would send to OSV, or none.
type ScanLine struct {
	ID      string   `json:"id"`
	Scanner string   `json:"scanner"`
	Package *Package `json:"package"`
}

type Verdict string

const (
	OK      Verdict = "OK"
	Partial Verdict = "PARTIAL"
	Wrong   Verdict = "WRONG"
	Skip    Verdict = "SKIP"
)

var pypiSeparators = regexp.MustCompile(`[-_.]+`)

func normalize(p Package) Package {
	if p.Ecosystem == "PyPI" {
		p.Name = pypiSeparators.ReplaceAllString(strings.ToLower(p.Name), "-")
	}
	return p
}

func normalizeAll(ps []Package) []Package {
	out := make([]Package, 0, len(ps))
	seen := map[Package]bool{}
	for _, p := range ps {
		n := normalize(p)
		if !seen[n] {
			seen[n] = true
			out = append(out, n)
		}
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].Ecosystem != out[j].Ecosystem {
			return out[i].Ecosystem < out[j].Ecosystem
		}
		return out[i].Name < out[j].Name
	})
	return out
}

func samePackages(a, b []Package) bool {
	a, b = normalizeAll(a), normalizeAll(b)
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// judge compares the one package a scanner checks with what the launcher really fetched.
// Lenient mode only asks whether the scanner parsed anything, which is what the
// scanners' own unit tests assert. It exists so the demo can prove its checks bite.
func judge(checked *Package, truth []Package, lenient bool) Verdict {
	if checked == nil {
		return Skip
	}
	if lenient {
		return OK
	}
	c := normalize(*checked)
	for _, t := range normalizeAll(truth) {
		if t == c {
			if len(normalizeAll(truth)) == 1 {
				return OK
			}
			return Partial
		}
	}
	return Wrong
}
