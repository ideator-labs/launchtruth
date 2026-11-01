package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"strings"
)

// CaseReport is one row of the report: the launch command, what the launcher really
// fetched, and each scanner's verdict.
type CaseReport struct {
	ID           string              `json:"id"`
	Kind         string              `json:"kind"`
	Command      string              `json:"command"`
	Requested    []Package           `json:"requested"`
	LabelMatches bool                `json:"label_matches"`
	Other        []string            `json:"other_requests"`
	Checked      map[string]*Package `json:"checked"`
	Verdicts     map[string]Verdict  `json:"verdicts"`
}

type Report struct {
	Lenient  bool         `json:"lenient"`
	Scanners []string     `json:"scanners"`
	Cases    []CaseReport `json:"cases"`
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: launchtruth truth|report [flags]")
		os.Exit(2)
	}
	var err error
	switch os.Args[1] {
	case "truth":
		err = runTruth(os.Args[2:])
	case "report":
		err = runReport(os.Args[2:])
	default:
		err = fmt.Errorf("unknown subcommand %q", os.Args[1])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "launchtruth:", err)
		os.Exit(1)
	}
}

func readJSON(path string, v any) error {
	b, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	return json.Unmarshal(b, v)
}

func writeJSON(path string, v any) error {
	b, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, append(b, '\n'), 0o644)
}

func runTruth(args []string) error {
	fs := flag.NewFlagSet("truth", flag.ExitOnError)
	casesPath := fs.String("cases", "corpus/cases.json", "launch command corpus")
	outPath := fs.String("out", "out/truth.json", "where to write measured truth")
	fs.Parse(args)
	var cases []Case
	if err := readJSON(*casesPath, &cases); err != nil {
		return err
	}
	truth, err := measureTruth(cases)
	if err != nil {
		return err
	}
	for _, t := range truth {
		fmt.Printf("%-28s requested %s\n", t.ID, names(t.Requested))
	}
	return writeJSON(*outPath, truth)
}

func runReport(args []string) error {
	fs := flag.NewFlagSet("report", flag.ExitOnError)
	casesPath := fs.String("cases", "corpus/cases.json", "launch command corpus")
	truthPath := fs.String("truth", "out/truth.json", "measured truth")
	scansPath := fs.String("scans", "out/scans.jsonl", "scanner answers, one JSON object per line")
	outPath := fs.String("out", "out/report.json", "where to write the report")
	lenient := fs.Bool("lenient", false, "count any parsed package as OK (the --break mode)")
	fs.Parse(args)

	var cases []Case
	if err := readJSON(*casesPath, &cases); err != nil {
		return err
	}
	var truths []Truth
	if err := readJSON(*truthPath, &truths); err != nil {
		return err
	}
	truthByID := map[string]Truth{}
	for _, t := range truths {
		truthByID[t.ID] = t
	}

	f, err := os.Open(*scansPath)
	if err != nil {
		return err
	}
	defer f.Close()
	checked := map[string]map[string]*Package{}
	var scanners []string
	seenScanner := map[string]bool{}
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" {
			continue
		}
		var s ScanLine
		if err := json.Unmarshal([]byte(line), &s); err != nil {
			return fmt.Errorf("bad scan line %q: %v", line, err)
		}
		if checked[s.ID] == nil {
			checked[s.ID] = map[string]*Package{}
		}
		checked[s.ID][s.Scanner] = s.Package
		if !seenScanner[s.Scanner] {
			seenScanner[s.Scanner] = true
			scanners = append(scanners, s.Scanner)
		}
	}
	if err := sc.Err(); err != nil {
		return err
	}

	rep := Report{Lenient: *lenient, Scanners: scanners}
	for _, c := range cases {
		t, ok := truthByID[c.ID]
		if !ok {
			return fmt.Errorf("no measured truth for %s", c.ID)
		}
		row := CaseReport{
			ID: c.ID, Kind: c.Kind,
			Command:      strings.Join(append([]string{c.Command}, c.Args...), " "),
			Requested:    t.Requested,
			LabelMatches: samePackages(c.Installs, t.Requested),
			Other:        append([]string{}, t.Other...),
			Checked:      map[string]*Package{},
			Verdicts:     map[string]Verdict{},
		}
		for _, s := range scanners {
			got, answered := checked[c.ID][s]
			if !answered {
				return fmt.Errorf("scanner %s gave no answer for %s", s, c.ID)
			}
			row.Checked[s] = got
			row.Verdicts[s] = judge(got, t.Requested, *lenient)
		}
		rep.Cases = append(rep.Cases, row)
	}
	printTable(rep)
	return writeJSON(*outPath, rep)
}

func names(ps []Package) string {
	var s []string
	for _, p := range ps {
		s = append(s, p.Ecosystem+":"+p.Name)
	}
	if len(s) == 0 {
		return "(nothing)"
	}
	return strings.Join(s, ", ")
}

func printTable(rep Report) {
	fmt.Printf("%-28s %-36s", "case", "launcher fetched")
	for _, s := range rep.Scanners {
		fmt.Printf(" %-24s", s)
	}
	fmt.Println()
	for _, r := range rep.Cases {
		fmt.Printf("%-28s %-36s", r.ID, names(r.Requested))
		for _, s := range rep.Scanners {
			cell := string(r.Verdicts[s])
			if p := r.Checked[s]; p != nil && r.Verdicts[s] != OK {
				cell += " " + p.Name
			}
			fmt.Printf(" %-24s", cell)
		}
		fmt.Println()
	}
}
