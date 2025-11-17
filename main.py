import sys

import matplotlib.pyplot as plt
import numpy as np

import charts
import read_pdf
from charts import format_segment_label
from utils.calculate_growth_rate import calculate_growth_rate
from utils.find_latest_pdf import get_latest_pdf

# Step 1: Extract data from the PDF
# Use provided PDF file or automatically find the latest one
pdf_file = sys.argv[1] if len(sys.argv) > 1 else get_latest_pdf()
print(f"Processing: {pdf_file}")
data = read_pdf.extract_data_from_pdf(pdf_file)

# Step 2: Assign data to variables
quarters = data["quarters"]
data_center = data["data_center"]
gaming = data["gaming"]
professional_visualization = data["professional_visualization"]
auto = data["auto"]
oem_other = data["oem_other"]
total = data["total"]

# Step 3: Calculate growth rates as percentages with + or -
growth_rates = [
    calculate_growth_rate(total[i], total[i - 1]) if i != 0 else 0
    for i in range(len(total))
]

# Step 4: Print growth rates
for quarter, rate in zip(quarters[1:], growth_rates[1:]):
    print(f"{quarter}: {rate}%")

# Step 5: Plotting - Stacked bars with total revenue line
x = np.arange(len(quarters))  # the label locations
width = 0.6  # the width of the bars

# Segment data for stacking (exclude total)
segment_labels = [
    "data_center",
    "gaming",
    "professional_visualization",
    "auto",
    "oem_other",
]
segment_data = [data_center, gaming, professional_visualization, auto, oem_other]

# Dynamic figure width: 3 inches per quarter, minimum 12 inches
fig_width = max(12, len(quarters) * 3)
fig, ax = plt.subplots(figsize=(fig_width, 8))

# Create stacked bars
bottom = np.zeros(len(quarters))
bars = []
for label, segment_values in zip(segment_labels, segment_data):
    bar = ax.bar(
        x, segment_values, width, label=format_segment_label(label), bottom=bottom
    )
    bars.append(bar)
    bottom += segment_values

# Step 6: Add total revenue and data center revenue as line overlays
ax2 = ax.twinx()  # Create second y-axis
line_total = ax2.plot(
    x,
    total,
    color="#2E2E2E",
    marker="o",
    linewidth=3,
    markersize=8,
    label="Total Revenue",
    zorder=5,
)

# Calculate data center growth rates
data_center_growth_rates = [
    calculate_growth_rate(data_center[i], data_center[i - 1]) if i != 0 else 0
    for i in range(len(data_center))
]

# Add data center revenue line (positioned at midpoint of data center segment)
data_center_midpoint = [val / 2 for val in data_center]
line_dc = ax2.plot(
    x,
    data_center_midpoint,
    color="#FFD700",
    marker="s",
    linewidth=2.5,
    markersize=7,
    label="Data Center Revenue",
    zorder=4,
)

# Make both y-axes have the same scale
ax2.set_ylim(ax.get_ylim())
ax2.set_ylabel("Revenue ($ in millions)", fontsize=12)

# Step 7: Add growth rate annotations on the total revenue line
for i, rate in enumerate(growth_rates):
    ax2.annotate(
        f"{rate}%",
        (x[i], total[i]),
        textcoords="offset points",
        xytext=(0, 15),
        ha="center",
        fontsize=10,
        fontweight="bold",
        bbox=dict(
            boxstyle="round,pad=0.3", facecolor="white", edgecolor="gray", alpha=0.8
        ),
    )

# Add growth rate annotations on the data center line
for i, rate in enumerate(data_center_growth_rates):
    ax2.annotate(
        f"{rate}%",
        (x[i], data_center_midpoint[i]),
        textcoords="offset points",
        xytext=(0, -25),
        ha="center",
        fontsize=9,
        fontweight="bold",
        color="#FFD700",
        bbox=dict(
            boxstyle="round,pad=0.3", facecolor="black", edgecolor="#FFD700", alpha=0.8
        ),
    )

# Step 8: Add labels, title, and styling
ax.set_xlabel("Quarter", fontsize=12)
ax.set_ylabel("Revenue by Segment ($ in millions)", fontsize=12)
ax.set_title(
    "NVIDIA Quarterly Revenue: Segment Breakdown & Total Trend",
    fontsize=14,
    fontweight="bold",
)
ax.set_xticks(x)
ax.set_xticklabels(quarters, rotation=0, ha="center", fontsize=11)

# Combine legends from both axes
handles1, labels1 = ax.get_legend_handles_labels()
handles2, labels2 = ax2.get_legend_handles_labels()
ax.legend(handles1 + handles2, labels1 + labels2, loc="upper left", fontsize=10)

# Adjust tick label size
ax.tick_params(axis="y", labelsize=10)
ax2.tick_params(axis="y", labelsize=10)

# Set Y-axis ticks with appropriate intervals
max_value = max(total)
y_ticks = np.arange(0, max_value + 5000, 5000)  # Intervals of 5,000
ax.set_yticks(y_ticks)
ax2.set_yticks(y_ticks)

# Add grid for better readability
ax.grid(axis="y", alpha=0.3, linestyle="--", linewidth=0.5)

# Step 9: Adjust layout and save the figure
fig.tight_layout()
plt.savefig("nvidia-revenue-trend.png", dpi=300, bbox_inches="tight")
plt.show()

# Step 10: Generate additional charts
print("\nGenerating additional analysis charts...")
charts.generate_market_share_chart(data)
charts.generate_stacked_area_chart(data)
charts.generate_segment_trend_lines(data)
charts.generate_growth_rate_comparison(data)
charts.generate_yoy_growth_chart(data)
charts.generate_cagr_chart(data)
charts.generate_revenue_contribution(data)
charts.generate_normalized_growth(data)
print("\nAll charts generated successfully!")
