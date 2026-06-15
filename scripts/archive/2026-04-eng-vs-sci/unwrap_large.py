"""Unwrap 10-bit Large counter overflow."""
import numpy as np


def unwrap_large(pho, large):
    """Correct Large counter 10-bit wrapping.

    Self-calibrates r = Large/PHO from low-rate data, then for each second:
      n_wraps = round((r * PHO - Large_obs) / 1024)
      Large_corrected = Large_obs + n_wraps * 1024

    Each second is corrected independently (no accumulation, no drift).
    At PHO < 16000, tolerance on r is ±0.032, well within same-observation
    stability (~±0.01), so n_wraps is always correct.

    Args:
        pho: PHO counts array (single detector)
        large: Large counts array (single detector)

    Returns:
        large_corrected: unwrapped Large counts
    """
    pho = pho.astype(float)
    large = large.astype(float)

    # Calibrate r from low-rate non-wrapping region
    low = (pho > 200) & (pho < 2500) & (large < 900)
    if low.sum() < 20:
        r = 0.3  # fallback
    else:
        r = np.median(large[low] / pho[low])

    predicted = r * pho
    n_wraps = np.round((predicted - large) / 1024).astype(int)
    n_wraps = np.maximum(n_wraps, 0)
    return large + n_wraps * 1024
