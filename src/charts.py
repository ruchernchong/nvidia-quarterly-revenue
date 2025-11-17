"""Chart generation functions for NVIDIA quarterly revenue analysis."""

from typing import Dict, List

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Wedge


def calculate_numeric_growth_rate(current: float, previous: float) -> float:
    """
    Calculate growth rate as a numeric percentage value.

    Args:
        current: Current period value
        previous: Previous period value

    Returns:
        Growth rate as a float percentage (e.g., 15.5 for 15.5%)
    """
    if previous == 0:
        return 0.0
    else:
        return ((current - previous) / previous) * 100


def format_segment_label(segment: str) -> str:
    """
    Convert snake_case segment names to display format.

    Args:
        segment: Segment name in snake_case (e.g., "data_center")

    Returns:
        Formatted display name (e.g., "Data Centre")
    """
    label_map = {
        "data_center": "Data Centre",
        "gaming": "Gaming",
        "professional_visualization": "Professional Visualisation",
        "auto": "Automotive",
        "oem_other": "OEM & Other",
    }
    return label_map.get(segment, segment.replace("_", " ").title())


def get_segment_colours() -> Dict[str, str]:
    """
    Get consistent colour scheme for revenue segments.

    Returns:
        Dictionary mapping segment names to hex colours
    """
    return {
        "data_center": "#76B900",  # NVIDIA green
        "gaming": "#1E88E5",  # Blue
        "professional_visualization": "#FFA726",  # Orange
        "auto": "#AB47BC",  # Purple
        "oem_other": "#78909C",  # Grey
    }


def generate_market_share_chart(
    data: Dict[str, List], output_file: str = "charts/market_share_chart.png"
) -> None:
    """
    Generate multi-panel pie/donut charts showing market share by segment for each quarter.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
    ]
    colours = get_segment_colours()

    # Create subplots - 2 rows, 4 columns for 8 quarters
    fig, axes = plt.subplots(2, 4, figsize=(20, 10))
    axes = axes.flatten()

    for idx, quarter in enumerate(quarters):
        ax = axes[idx]

        # Get revenue values for this quarter
        values = [data[segment][idx] for segment in segments]
        labels = [format_segment_label(seg) for seg in segments]
        segment_colours = [colours[seg] for seg in segments]

        # Create donut chart
        wedges, texts, autotexts = ax.pie(
            values,
            labels=labels,
            colors=segment_colours,
            autopct=lambda pct: f"{pct:.1f}%" if pct > 5 else "",
            startangle=90,
            pctdistance=0.85,
            wedgeprops=dict(width=0.5),
        )

        # Styling
        for text in texts:
            text.set_fontsize(9)
        for autotext in autotexts:
            autotext.set_color("white")
            autotext.set_fontsize(9)
            autotext.set_fontweight("bold")

        ax.set_title(f"{quarter}", fontsize=12, fontweight="bold")

    # Remove title and adjust layout
    fig.suptitle(
        "NVIDIA Quarterly Revenue: Market Share by Segment",
        fontsize=16,
        fontweight="bold",
        y=0.98,
    )
    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_stacked_area_chart(
    data: Dict[str, List], output_file: str = "charts/stacked_area_chart.png"
) -> None:
    """
    Generate 100% stacked area chart showing relative market share over time.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
    ]
    colours = get_segment_colours()

    # Calculate percentages for each quarter
    totals = data["total"]
    percentages = []

    for segment in segments:
        segment_pct = [
            (data[segment][i] / totals[i]) * 100 for i in range(len(quarters))
        ]
        percentages.append(segment_pct)

    # Create figure
    fig_width = max(14, len(quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    # Create stacked area chart
    ax.stackplot(
        range(len(quarters)),
        *percentages,
        labels=[format_segment_label(seg) for seg in segments],
        colors=[colours[seg] for seg in segments],
        alpha=0.8,
    )

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("Market Share (%)", fontsize=12)
    ax.set_title(
        "NVIDIA Revenue: Market Share Evolution (100% Stacked)",
        fontsize=14,
        fontweight="bold",
    )
    ax.set_xticks(range(len(quarters)))
    ax.set_xticklabels(quarters, rotation=45, ha="right", fontsize=11)
    ax.set_ylim(0, 100)
    ax.legend(loc="upper left", fontsize=10)
    ax.grid(axis="y", alpha=0.3, linestyle="--", linewidth=0.5)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_segment_trend_lines(
    data: Dict[str, List], output_file: str = "charts/segment_trends.png"
) -> None:
    """
    Generate line chart showing individual segment revenue trends over time.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
    ]
    colours = get_segment_colours()

    fig_width = max(14, len(quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    x = range(len(quarters))

    # Plot each segment
    for segment in segments:
        ax.plot(
            x,
            data[segment],
            marker="o",
            linewidth=2.5,
            markersize=7,
            label=format_segment_label(segment),
            color=colours[segment],
        )

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("Revenue ($ in millions)", fontsize=12)
    ax.set_title(
        "NVIDIA Quarterly Revenue: Segment Trends", fontsize=14, fontweight="bold"
    )
    ax.set_xticks(x)
    ax.set_xticklabels(quarters, rotation=45, ha="right", fontsize=11)
    ax.legend(loc="upper left", fontsize=10)
    ax.grid(axis="both", alpha=0.3, linestyle="--", linewidth=0.5)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_growth_rate_comparison(
    data: Dict[str, List], output_file: str = "charts/growth_rate_qoq.png"
) -> None:
    """
    Generate bar chart comparing quarter-over-quarter growth rates across segments.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
    ]
    colours = get_segment_colours()

    # Calculate growth rates for each segment
    growth_data = {}
    for segment in segments:
        segment_data = data[segment]
        growth_rates = [
            (
                calculate_numeric_growth_rate(segment_data[i], segment_data[i - 1])
                if i != 0
                else 0.0
            )
            for i in range(len(segment_data))
        ]
        growth_data[segment] = growth_rates

    # Create figure
    fig_width = max(14, len(quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    x = np.arange(len(quarters))
    width = 0.15
    multiplier = 0

    # Create grouped bars
    for segment in segments:
        offset = width * multiplier
        bars = ax.bar(
            x + offset,
            growth_data[segment],
            width,
            label=format_segment_label(segment),
            color=colours[segment],
        )
        multiplier += 1

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("Growth Rate (%)", fontsize=12)
    ax.set_title(
        "NVIDIA Quarterly Revenue: Q/Q Growth Rate Comparison",
        fontsize=14,
        fontweight="bold",
    )
    ax.set_xticks(x + width * 2)
    ax.set_xticklabels(quarters, rotation=45, ha="right", fontsize=11)
    ax.legend(loc="upper left", fontsize=9)
    ax.grid(axis="y", alpha=0.3, linestyle="--", linewidth=0.5)
    ax.axhline(y=0, color="black", linestyle="-", linewidth=0.8)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_yoy_growth_chart(
    data: Dict[str, List], output_file: str = "charts/growth_rate_yoy.png"
) -> None:
    """
    Generate bar chart comparing year-over-year growth rates across segments.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
    ]
    colours = get_segment_colours()

    # Calculate YoY growth (compare to 4 quarters ago)
    growth_data = {}
    yoy_quarters = []

    for segment in segments:
        segment_data = data[segment]
        growth_rates = []
        for i in range(len(segment_data)):
            if i >= 4:  # Need at least 4 quarters for YoY
                yoy_growth = calculate_numeric_growth_rate(
                    segment_data[i], segment_data[i - 4]
                )
                growth_rates.append(yoy_growth)
                if segment == "data_center":  # Only add quarters once
                    yoy_quarters.append(quarters[i])
            else:
                growth_rates.append(0.0)
        growth_data[segment] = growth_rates

    # Only plot quarters with YoY data available
    if len(yoy_quarters) == 0:
        print(f"Skipped: {output_file} (insufficient data for YoY comparison)")
        return

    # Create figure
    fig_width = max(12, len(yoy_quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    x = np.arange(len(yoy_quarters))
    width = 0.15
    multiplier = 0

    # Create grouped bars (only for quarters with YoY data)
    for segment in segments:
        offset = width * multiplier
        yoy_values = [growth_data[segment][i] for i in range(4, len(quarters))]
        bars = ax.bar(
            x + offset,
            yoy_values,
            width,
            label=format_segment_label(segment),
            color=colours[segment],
        )
        multiplier += 1

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("Growth Rate (%)", fontsize=12)
    ax.set_title(
        "NVIDIA Quarterly Revenue: Y/Y Growth Rate Comparison",
        fontsize=14,
        fontweight="bold",
    )
    ax.set_xticks(x + width * 2)
    ax.set_xticklabels(yoy_quarters, rotation=45, ha="right", fontsize=11)
    ax.legend(loc="upper left", fontsize=9)
    ax.grid(axis="y", alpha=0.3, linestyle="--", linewidth=0.5)
    ax.axhline(y=0, color="black", linestyle="-", linewidth=0.8)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_cagr_chart(
    data: Dict[str, List], output_file: str = "charts/cagr_chart.png"
) -> None:
    """
    Generate line chart showing Compound Annual Growth Rate (CAGR) over quarters.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
        "total",
    ]
    colours = get_segment_colours()
    colours["total"] = "#2E2E2E"  # Dark grey for total

    # Create figure
    fig_width = max(14, len(quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    x = range(len(quarters))

    # Calculate CAGR at each quarter point (from baseline to that quarter)
    for segment in segments:
        segment_data = data[segment]
        baseline_value = segment_data[0]
        cagr_over_time = []

        for i, value in enumerate(segment_data):
            if i == 0:
                cagr_over_time.append(0)  # No growth at baseline
            else:
                # CAGR from baseline to current quarter
                num_quarters = i
                num_years = num_quarters / 4
                if baseline_value > 0:
                    cagr = ((value / baseline_value) ** (1 / num_years) - 1) * 100
                    cagr_over_time.append(cagr)
                else:
                    cagr_over_time.append(0)

        label = format_segment_label(segment) if segment != "total" else "Total"
        line_width = 3 if segment == "total" else 2.5
        marker_size = 8 if segment == "total" else 7

        ax.plot(
            x,
            cagr_over_time,
            marker="o",
            linewidth=line_width,
            markersize=marker_size,
            label=label,
            color=colours.get(segment, "#CCCCCC"),
        )

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("CAGR (%)", fontsize=12)
    ax.set_title(
        f"NVIDIA Revenue: Compound Annual Growth Rate from {quarters[0]}",
        fontsize=14,
        fontweight="bold",
    )
    ax.set_xticks(x)
    ax.set_xticklabels(quarters, rotation=45, ha="right", fontsize=11)
    ax.legend(loc="upper left", fontsize=10)
    ax.grid(axis="both", alpha=0.3, linestyle="--", linewidth=0.5)
    ax.axhline(y=0, color="grey", linestyle="--", linewidth=1, alpha=0.5)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_revenue_contribution(
    data: Dict[str, List], output_file: str = "charts/revenue_contribution.png"
) -> None:
    """
    Generate chart showing each segment's contribution to total revenue growth.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
    ]
    colours = get_segment_colours()

    # Calculate absolute change in revenue for each segment
    contributions = {seg: [] for seg in segments}
    contribution_quarters = []

    for i in range(1, len(quarters)):
        total_growth = data["total"][i] - data["total"][i - 1]
        contribution_quarters.append(quarters[i])

        for segment in segments:
            segment_growth = data[segment][i] - data[segment][i - 1]
            # Convert to percentage of total growth
            if total_growth != 0:
                contribution_pct = (segment_growth / total_growth) * 100
                contributions[segment].append(contribution_pct)
            else:
                contributions[segment].append(0)

    # Create figure
    fig_width = max(14, len(contribution_quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    x = np.arange(len(contribution_quarters))
    width = 0.15
    multiplier = 0

    # Create grouped bars
    for segment in segments:
        offset = width * multiplier
        bars = ax.bar(
            x + offset,
            contributions[segment],
            width,
            label=format_segment_label(segment),
            color=colours[segment],
        )
        multiplier += 1

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("Contribution to Total Growth (%)", fontsize=12)
    ax.set_title(
        "NVIDIA Revenue: Segment Contribution to Total Growth",
        fontsize=14,
        fontweight="bold",
    )
    ax.set_xticks(x + width * 2)
    ax.set_xticklabels(contribution_quarters, rotation=45, ha="right", fontsize=11)
    ax.legend(loc="upper left", fontsize=9)
    ax.grid(axis="y", alpha=0.3, linestyle="--", linewidth=0.5)
    ax.axhline(y=0, color="black", linestyle="-", linewidth=0.8)
    ax.axhline(y=100, color="grey", linestyle="--", linewidth=0.8, alpha=0.5)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")


def generate_normalized_growth(
    data: Dict[str, List], output_file: str = "charts/normalized_growth.png"
) -> None:
    """
    Generate chart with all segments indexed to 100 at baseline quarter.

    Args:
        data: Dictionary containing quarterly revenue data
        output_file: Output filename for the chart
    """
    quarters = data["quarters"]
    segments = [
        "data_center",
        "gaming",
        "professional_visualization",
        "auto",
        "oem_other",
        "total",
    ]
    colours = get_segment_colours()
    colours["total"] = "#2E2E2E"  # Dark grey for total

    fig_width = max(14, len(quarters) * 2)
    fig, ax = plt.subplots(figsize=(fig_width, 8))

    x = range(len(quarters))

    # Normalize each segment to baseline (first quarter = 100)
    for segment in segments:
        segment_data = data[segment]
        baseline = segment_data[0]
        normalized = [(value / baseline) * 100 for value in segment_data]

        label = format_segment_label(segment) if segment != "total" else "Total"
        line_width = 3 if segment == "total" else 2.5
        marker_size = 8 if segment == "total" else 7

        ax.plot(
            x,
            normalized,
            marker="o",
            linewidth=line_width,
            markersize=marker_size,
            label=label,
            color=colours.get(segment, "#CCCCCC"),
        )

    # Styling
    ax.set_xlabel("Quarter", fontsize=12)
    ax.set_ylabel("Indexed Revenue (Baseline = 100)", fontsize=12)
    ax.set_title(
        f"NVIDIA Revenue: Normalized Growth (Indexed to {quarters[0]})",
        fontsize=14,
        fontweight="bold",
    )
    ax.set_xticks(x)
    ax.set_xticklabels(quarters, rotation=45, ha="right", fontsize=11)
    ax.legend(loc="upper left", fontsize=10)
    ax.grid(axis="both", alpha=0.3, linestyle="--", linewidth=0.5)
    ax.axhline(y=100, color="grey", linestyle="--", linewidth=1, alpha=0.5)

    plt.tight_layout()
    plt.savefig(output_file, dpi=300, bbox_inches="tight")
    plt.close()
    print(f"Saved: {output_file}")
