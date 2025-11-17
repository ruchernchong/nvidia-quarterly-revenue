import sys

import matplotlib.pyplot as plt
import numpy as np

import read_pdf
from utils.calculate_growth_rate import calculate_growth_rate
from utils.find_latest_pdf import get_latest_pdf
from utils.replace_text import replace_text

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

# Step 5: Plotting
x = np.arange(len(quarters))  # the label locations
width = 0.15  # the width of the bars
bar_positions = [x - 2 * width, x - width, x, x + width, x + 2 * width, x + 3 * width]
bar_labels = [
    "data_center",
    "gaming",
    "professional_visualization",
    "auto",
    "oem_other",
    "total",
]
bar_data = [data_center, gaming, professional_visualization, auto, oem_other, total]

# Dynamic figure width: 2.5 inches per quarter, minimum 14 inches
fig_width = max(14, len(quarters) * 2.5)
fig, ax = plt.subplots(figsize=(fig_width, 8))

rects = []
for pos, label, data in zip(bar_positions, bar_labels, bar_data):
    rect = ax.bar(pos, data, width, label=replace_text(label))
    rects.append(rect)

# Step 6: Add growth rate annotations
for i, rate in enumerate(growth_rates):
    ax.annotate(
        f"{rate}%",
        (x[i], total[i]),
        textcoords="offset points",
        xytext=(0, 8),
        ha="center",
        fontsize=9,
        fontweight="bold",
    )

# Step 7: Add some text for labels, title, and custom x-axis tick labels, etc.
ax.set_xlabel("Quarter", fontsize=12)
ax.set_ylabel("Revenue ($ in millions)", fontsize=12)
ax.set_title("NVIDIA Quarterly Revenue Trend by Market", fontsize=14, fontweight="bold")
ax.set_xticks(x)
ax.set_xticklabels(quarters, rotation=45, ha="right", fontsize=10)
ax.legend(fontsize=10)

# Adjust tick label size
ax.tick_params(axis="y", labelsize=10)

# Step 8: Adjust layout and save the figure
fig.tight_layout()
plt.savefig("nvidia-revenue-trend.png")
plt.show()
