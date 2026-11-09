"""Independent AC16 arithmetic from the reviewed F227 finite-binary32 contract.

No Ocelli imports, process calls, expected-result capture or study I/O.
PS3.3 C.11.2.1.3.2 specifies the LINEAR_EXACT piecewise function.
The F227 plan specifies the sample percentiles and finite search policy.
"""
import numpy as np

F = np.float32


def step(value, direction):
    return np.nextafter(F(value), F(direction), dtype=np.float32)


def moved(value, direction, count):
    for _ in range(count):
        value = step(value, direction)
    return value


def display(value, center, width):
    """Round each arithmetic operation separately, with no fused multiply-add."""
    value, center, width = F(value), F(center), F(width)
    if not all(np.isfinite(x) for x in (value, center, width)) or width <= 0:
        return F(np.nan)
    half = F(width / F(2))
    if value <= F(center - half):
        return F(0)
    if value > F(center + half):
        return F(255)
    quotient = F(F(value - center) / width)
    return F(F(F(quotient + F(0.5)) * F(255)) + F(0))


def _search(first, widening, center_of, accepted):
    first = F(first)
    center = F(center_of(first))
    if accepted(center, first):
        return center, first
    neighbor = step(first, np.inf if widening else -np.inf)
    unit = F(neighbor - first) if widening else F(first - neighbor)
    for k in range(48):
        change = F(np.ldexp(unit, k))
        width = F(first + change) if widening else F(first - change)
        if not np.isfinite(width) or width <= 0:
            break
        center = F(center_of(width))
        if accepted(center, width):
            return center, width
    raise ValueError("unverifiable-window")


def reference_window(low=None, high=None, *, excluded_max=None):
    """Return (center, width, basis) or a closed refusal, never an approximation.

    low/high are independently obtained nearest-rank modality percentiles.
    With no counted values, pass only the maximum excluded modality value.
    Numeric results must remain local when the input is an operator study.
    """
    with np.errstate(all="ignore"):
        if low is None and high is None:
            if excluded_max is None or not np.isfinite(F(excluded_max)):
                raise ValueError("empty-or-nonfinite-pool")
            maximum = F(excluded_max)
            center, width = _search(
                F(1), True, lambda w: F(maximum + F(w / F(2))),
                lambda c, w: display(maximum, c, w) == F(0),
            )
            return center, width, "background"
        if low is None or high is None:
            raise ValueError("invalid-percentiles")
        low, high = F(low), F(high)
        if not np.isfinite(low) or not np.isfinite(high) or low > high:
            raise ValueError("invalid-percentiles")
        span = F(high - low)
        midpoint = low if span == 0 else F(low + F(span / F(2)))
        if not np.isfinite(span) or not np.isfinite(midpoint):
            raise ValueError("nonfinite-window")
        close = high <= step(step(low, np.inf), np.inf)
        if span == 0 or close:
            points = (moved(low, -np.inf, 16), low, high, moved(high, np.inf, 16))

            def middle(c, w):
                if display(midpoint, c, w) != F(127.5):
                    return False
                return all(F(127) < display(x, c, w) < F(128) for x in points)

            center, width = _search(F(1), True, lambda _: midpoint, middle)
            return center, width, "constant"
        lower, upper = low, high
        for _ in range(16):
            a, b = step(lower, np.inf), step(upper, -np.inf)
            if a >= b:
                break
            lower, upper = a, b

        def extremes(c, w):
            return (all(display(x, c, w) == F(0) for x in (low, lower))
                    and all(display(x, c, w) == F(255) for x in (upper, high)))

        center, width = _search(
            F(upper - lower), False, lambda w: F(lower + F(w / F(2))), extremes,
        )
        return center, width, "percentile"
