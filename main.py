import matplotlib.pyplot as plt
import numpy as np

from utils.calculate_growth_rate import calculate_growth_rate
from utils.replace_text import replace_text

# Data
quarters = ["Q1 FY24", "Q2 FY24", "Q3 FY24", "Q4 FY24", "Q1 FY25"]
data_center = [4284, 10323, 14514, 18404, 22563]
gaming = [2240, 2486, 2856, 2865, 2647]
professional_visualization = [295, 379, 416, 463, 427]
auto = [296, 253, 261, 281, 329]
oem_other = [77, 66, 73, 90, 78]
total = [7192, 13507, 18120, 22103, 26044]

# Calculate growth rates as percentages with + or -
growth_rates = [calculate_growth_rate(total[i], total[i - 1]) for i in range(len(total))]

# Print growth rates
for quarter, rate in zip(quarters[1:], growth_rates[1:]):
    print(f"{quarter}: {rate}")

# Plotting
x = np.arange(len(quarters))  # the label locations
width = 0.1  # the width of the bars
bar_positions = [x - 2 * width, x - width, x, x + width, x + 2 * width, x + 3 * width]
bar_labels = ['Data Center', 'Gaming', 'Professional Visualization', 'Auto', 'OEM & Other', 'Total']

fig, ax = plt.subplots(figsize=(14, 8))

rects = []
for pos, label in zip(bar_positions, bar_labels):
    rect = ax.bar(pos, eval(replace_text(label)), width, label=label)
    rects.append(rect)

# Add growth rate annotations
for i, rate in enumerate(growth_rates):
    ax.annotate(f'{rate}', (x[i], total[i]), textcoords="offset points", xytext=(0, 0), ha='center')

# Add some text for labels, title and custom x-axis tick labels, etc.
ax.set_xlabel('Quarter')
ax.set_ylabel('Revenue ($ in millions)')
ax.set_title('NVIDIA Quarterly Revenue Trend by Market')
ax.set_xticks(x)
ax.set_xticklabels(quarters)
ax.legend()

# Rotate the tick labels for better readability
plt.xticks(rotation=45)

fig.tight_layout()

plt.savefig('nvidia-revenue-trend.png')
plt.show()
