using Printf

struct RevenueRow
    quarter::String
    values::Vector{Int}
end

number(s) = parse(Int, replace(s, "\$" => "", "," => ""))

function read_csv(path)
    lines = readlines(path)
    !isempty(lines) && startswith(lines[1], "quarter,") || error("expected repository revenue CSV")
    rows = RevenueRow[]
    for line in lines[2:end]
        occursin('"', line) && error("quoted CSV fields are outside this prototype's scope")
        fields = split(line, ',')
        length(fields) >= 9 || error("invalid revenue CSV row")
        push!(rows, RevenueRow(fields[1], number.(fields[4:9])))
    end
    rows
end

function read_pdf(path)
    text = read(`pdftotext -f 1 -l 1 -layout $path -`, String)
    quarters = String[]
    segments = Dict{Int,Vector{Int}}()
    segment = 0
    labels = ["Data Center", "Gaming", "Professional", "Auto", "OEM & Other", "TOTAL"]
    for line in split(text, '\n')
        fields = split(line)
        if occursin("(\$ in millions)", line)
            for i in 1:length(fields)-1
                if startswith(fields[i], "Q") && startswith(fields[i+1], "FY")
                    push!(quarters, fields[i] * " " * fields[i+1])
                end
            end
            continue
        end
        for (i, label) in enumerate(labels)
            startswith(strip(line), label) && (segment = i)
        end
        (segment == 0 || isempty(quarters) || length(fields) < length(quarters)) && continue
        values = tryparse.(Int, replace.(fields[end-length(quarters)+1:end], "\$" => "", "," => ""))
        if all(!isnothing, values)
            haskey(segments, segment) && error("duplicate segment")
            segments[segment] = Int.(values)
            segment = 0
        end
    end
    !isempty(quarters) && length(segments) == 6 || error("expected six revenue rows on page one")
    [RevenueRow(quarters[i], [segments[j][i] for j in 1:6]) for i in reverse(eachindex(quarters))]
end

function run(args)
    length(args) == 2 || error("usage: julia main.jl INPUT.csv|INPUT.pdf OUTPUT_DIRECTORY")
    rows = endswith(lowercase(args[1]), ".pdf") ? read_pdf(args[1]) : read_csv(args[1])
    for row in rows
        all(>=(0), row.values) && sum(row.values[1:5]) == row.values[6] || error("invalid segment total: " * row.quarter)
    end
    isempty(rows) && error("no positive revenue")
    max_total = maximum(row.values[6] for row in rows)
    max_total > 0 || error("no positive revenue")
    mkpath(args[2])
    report, svg = IOBuffer(), IOBuffer()
    println(report, "quarter,data_center,gaming,professional_visualization,automotive,oem_other,total_revenue,qoq_percent")
    println(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"600\" viewBox=\"0 0 1200 600\"><rect width=\"1200\" height=\"600\" fill=\"white\"/><text x=\"60\" y=\"30\">NVIDIA revenue by market (\$ millions)</text>")
    colours = ["#76b900", "#2563eb", "#a855f7", "#f59e0b", "#64748b"]
    labels = ["Data Centre", "Gaming", "Professional Visualisation", "Automotive", "OEM &amp; Other"]
    for j in 1:5
        @printf(svg, "<text x=\"%d\" y=\"580\" fill=\"%s\">%s</text>\n", 60+(j-1)*215, colours[j], labels[j])
    end
    step = 1080.0 / length(rows)
    for (i, row) in enumerate(rows)
        growth = i > 1 && rows[i-1].values[6] != 0 ? (row.values[6]-rows[i-1].values[6])/rows[i-1].values[6]*100 : 0.0
        print(report, row.quarter, ",", join(row.values, ","))
        @printf(report, ",%.6f\n", growth)
        x, bottom = 60+(i-1)*step, 520.0
        for j in 1:5
            h = row.values[j] / max_total * 430
            bottom -= h
            @printf(svg, "<rect x=\"%.3f\" y=\"%.3f\" width=\"%.3f\" height=\"%.3f\" fill=\"%s\"/>\n", x, bottom, step*0.75, h, colours[j])
        end
        q = replace(row.quarter, "&" => "&amp;", "<" => "&lt;", ">" => "&gt;", "\"" => "&quot;", "'" => "&#39;")
        @printf(svg, "<text x=\"%.3f\" y=\"%.3f\" font-size=\"10\">%d</text><text transform=\"translate(%.3f 535) rotate(30)\" font-size=\"10\">%s</text>\n", x, bottom-8, row.values[6], x, q)
    end
    println(svg, "</svg>")
    write(joinpath(args[2], "analysis.csv"), take!(report))
    write(joinpath(args[2], "revenue.svg"), take!(svg))
end

try
    run(ARGS)
catch e
    showerror(stderr, e)
    println(stderr)
    exit(1)
end
