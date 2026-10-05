package main

import (
	"encoding/csv"
	"fmt"
	"html"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
)

type row struct {
	quarter string
	values  [6]int
}

func number(s string) (int, error) {
	return strconv.Atoi(strings.NewReplacer("$", "", ",", "").Replace(s))
}

func readCSV(path string) ([]row, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	records, err := csv.NewReader(f).ReadAll()
	if err != nil {
		return nil, err
	}
	if len(records) < 2 || len(records[0]) < 9 || records[0][0] != "quarter" {
		return nil, fmt.Errorf("expected repository revenue CSV")
	}
	rows := make([]row, 0, len(records)-1)
	for _, fields := range records[1:] {
		r := row{quarter: fields[0]}
		for i := range r.values {
			r.values[i], err = number(fields[i+3])
			if err != nil {
				return nil, err
			}
		}
		rows = append(rows, r)
	}
	return rows, nil
}

func readPDF(path string) ([]row, error) {
	text, err := exec.Command("pdftotext", "-f", "1", "-l", "1", "-layout", path, "-").Output()
	if err != nil {
		return nil, err
	}
	var quarters []string
	segments := make([][]int, 6)
	segment := -1
	labels := []string{"Data Center", "Gaming", "Professional", "Auto", "OEM & Other", "TOTAL"}
	for _, line := range strings.Split(string(text), "\n") {
		fields := strings.Fields(line)
		if strings.Contains(line, "($ in millions)") {
			for i := 0; i+1 < len(fields); i++ {
				if strings.HasPrefix(fields[i], "Q") && strings.HasPrefix(fields[i+1], "FY") {
					quarters = append(quarters, fields[i]+" "+fields[i+1])
				}
			}
			continue
		}
		for i, label := range labels {
			if strings.HasPrefix(strings.TrimSpace(line), label) {
				segment = i
			}
		}
		if segment < 0 || len(quarters) == 0 || len(fields) < len(quarters) {
			continue
		}
		values := make([]int, len(quarters))
		valid := true
		for i, field := range fields[len(fields)-len(quarters):] {
			values[i], err = number(field)
			if err != nil {
				valid = false
				break
			}
		}
		if valid {
			if segments[segment] != nil {
				return nil, fmt.Errorf("duplicate segment")
			}
			segments[segment] = values
			segment = -1
		}
	}
	if len(quarters) == 0 {
		return nil, fmt.Errorf("expected six revenue rows on page one")
	}
	for _, values := range segments {
		if len(values) != len(quarters) {
			return nil, fmt.Errorf("missing segment")
		}
	}
	rows := make([]row, len(quarters))
	for i := range rows {
		j := len(rows) - 1 - i
		rows[i].quarter = quarters[j]
		for k := range rows[i].values {
			rows[i].values[k] = segments[k][j]
		}
	}
	return rows, nil
}

func run() error {
	if len(os.Args) != 3 {
		return fmt.Errorf("usage: revenue INPUT.csv|INPUT.pdf OUTPUT_DIRECTORY")
	}
	var rows []row
	var err error
	if strings.HasSuffix(strings.ToLower(os.Args[1]), ".pdf") {
		rows, err = readPDF(os.Args[1])
	} else {
		rows, err = readCSV(os.Args[1])
	}
	if err != nil {
		return err
	}
	maxTotal := 0
	for _, r := range rows {
		sum := 0
		for _, n := range r.values[:5] {
			if n < 0 {
				return fmt.Errorf("negative revenue")
			}
			sum += n
		}
		if sum != r.values[5] {
			return fmt.Errorf("segment total mismatch: %s", r.quarter)
		}
		if sum > maxTotal {
			maxTotal = sum
		}
	}
	if maxTotal <= 0 {
		return fmt.Errorf("no positive revenue")
	}
	if err = os.MkdirAll(os.Args[2], 0755); err != nil {
		return err
	}
	var report, svg strings.Builder
	report.WriteString("quarter,data_center,gaming,professional_visualization,automotive,oem_other,total_revenue,qoq_percent\n")
	svg.WriteString("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"600\" viewBox=\"0 0 1200 600\"><rect width=\"1200\" height=\"600\" fill=\"white\"/><text x=\"60\" y=\"30\">NVIDIA revenue by market ($ millions)</text>\n")
	colours := [5]string{"#76b900", "#2563eb", "#a855f7", "#f59e0b", "#64748b"}
	labels := [5]string{"Data Centre", "Gaming", "Professional Visualisation", "Automotive", "OEM &amp; Other"}
	for j, label := range labels {
		fmt.Fprintf(&svg, "<text x=\"%d\" y=\"580\" fill=\"%s\">%s</text>\n", 60+j*215, colours[j], label)
	}
	step := 1080.0 / float64(len(rows))
	for i, r := range rows {
		growth := 0.0
		if i > 0 && rows[i-1].values[5] != 0 {
			growth = float64(r.values[5]-rows[i-1].values[5]) / float64(rows[i-1].values[5]) * 100
		}
		fmt.Fprintf(&report, "%s", r.quarter)
		for _, n := range r.values {
			fmt.Fprintf(&report, ",%d", n)
		}
		fmt.Fprintf(&report, ",%.6f\n", growth)
		x, bottom := 60+float64(i)*step, 520.0
		for j, n := range r.values[:5] {
			h := float64(n) / float64(maxTotal) * 430
			bottom -= h
			fmt.Fprintf(&svg, "<rect x=\"%.3f\" y=\"%.3f\" width=\"%.3f\" height=\"%.3f\" fill=\"%s\"/>\n", x, bottom, step*.75, h, colours[j])
		}
		fmt.Fprintf(&svg, "<text x=\"%.3f\" y=\"%.3f\" font-size=\"10\">%d</text><text transform=\"translate(%.3f 535) rotate(30)\" font-size=\"10\">%s</text>\n", x, bottom-8, r.values[5], x, html.EscapeString(r.quarter))
	}
	svg.WriteString("</svg>\n")
	if err = os.WriteFile(filepath.Join(os.Args[2], "analysis.csv"), []byte(report.String()), 0644); err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(os.Args[2], "revenue.svg"), []byte(svg.String()), 0644)
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
