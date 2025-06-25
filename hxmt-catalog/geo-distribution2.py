import sqlite3

import cartopy.crs as ccrs
import matplotlib.pyplot as plt
import numpy as np

conn = sqlite3.connect("blink.db")
cursor = conn.cursor()
cursor.execute(
    """
    SELECT longitude, latitude, associated_lightning_count
    FROM signal
    WHERE start < '2025-01-01'
        AND (fp_year < 1e-5 OR (fp_year < 1 AND associated_lightning_count > 0));
    """
)
signals = cursor.fetchall()
cursor.close()
conn.close()

cm = 1 / 2.54
plt.figure(figsize=(20 * cm, 7 * cm), dpi=1200)
