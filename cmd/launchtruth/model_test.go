package main

import "testing"

func TestPackageFromPath(t *testing.T) {
	cases := map[string]*Package{
		"/mcp-evil":             {Ecosystem: "npm", Name: "mcp-evil"},
		"/@acme%2fmcp-server":   {Ecosystem: "npm", Name: "@acme/mcp-server"},
		"/@acme%2Fmcp-server":   {Ecosystem: "npm", Name: "@acme/mcp-server"},
		"/simple/mcp-evil/":     {Ecosystem: "PyPI", Name: "mcp-evil"},
		"/-/npm/v1/security/x":  nil,
		"/simple/":              nil,
		"/mcp-evil/-/x-1.0.tgz": nil,
	}
	for path, want := range cases {
		got, ok := packageFromPath(path)
		if want == nil {
			if ok {
				t.Errorf("%s: got %v, want no package", path, got)
			}
			continue
		}
		if !ok || got != *want {
			t.Errorf("%s: got %v %v, want %v", path, got, ok, *want)
		}
	}
}

func TestJudge(t *testing.T) {
	evil := Package{Ecosystem: "PyPI", Name: "mcp-evil"}
	good := Package{Ecosystem: "PyPI", Name: "mcp-server-good"}
	unnormalized := Package{Ecosystem: "PyPI", Name: "Mcp_Server.Good"}
	tests := []struct {
		name    string
		checked *Package
		truth   []Package
		lenient bool
		want    Verdict
	}{
		{"exact", &good, []Package{good}, false, OK},
		{"pep503 names match", &unnormalized, []Package{good}, false, OK},
		{"wrong package", &good, []Package{evil}, false, Wrong},
		{"one of two", &good, []Package{evil, good}, false, Partial},
		{"nothing checked", nil, []Package{evil}, false, Skip},
		{"ecosystem matters", &Package{Ecosystem: "npm", Name: "mcp-evil"}, []Package{evil}, false, Wrong},
		{"lenient accepts wrong", &good, []Package{evil}, true, OK},
		{"lenient still skips", nil, []Package{evil}, true, Skip},
	}
	for _, tc := range tests {
		if got := judge(tc.checked, tc.truth, tc.lenient); got != tc.want {
			t.Errorf("%s: got %s, want %s", tc.name, got, tc.want)
		}
	}
}
