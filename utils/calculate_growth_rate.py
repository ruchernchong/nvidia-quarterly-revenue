def calculate_growth_rate(current: float, previous: float) -> str:
    if previous == 0:
        return '0.00%'
    else:
        growth_rate: float = ((current - previous) / previous) * 100
        return f'+{growth_rate:.2f}%' if growth_rate > 0 else f'{growth_rate:.2f}%'
