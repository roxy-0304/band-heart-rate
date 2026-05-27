import struct

# Create a minimal valid ICO file (32x32, 32-bit BGRA)
width, height = 32, 32

# Generate pixels: red heart on transparent background
pixels = bytearray()
for y in range(32):
    for x in range(32):
        hx = (x - 16) / 12.0
        hy = (y - 16) / 12.0
        heart = (hx*hx + hy*hy - 1)**3 - hx*hx * hy*hy*hy
        if heart < 0:
            pixels.extend([0, 0, 255, 255])  # BGRA - red
        else:
            pixels.extend([0, 0, 0, 0])      # transparent

# AND mask (1 bit per pixel, 0=opaque, 1=transparent)
# For 32x32, needs 32 bytes per row, padded to 4-byte boundary
and_mask = bytearray(32 * 4)  # 128 bytes for 32 rows of 32 bits

# BMP info header for 32bpp
# Format: size(I) width(I) height(I) planes(H) bpp(H) compression(I) imageSize(I) xPPM(I) yPPM(I) clrUsed(I) clrImportant(I)
bmp_header = struct.pack('<IIIHHIIIIII',
    40,             # size
    width,           # width
    height * 2,     # height (2x for ICO format means actual height = 32)
    1,              # planes
    32,             # bpp
    0,              # compression (BI_RGB)
    len(pixels),    # imageSize
    0,              # x pixels per meter
    0,              # y pixels per meter
    0,              # colors used
    0               # colors important
)

# ICO directory entry
icon_data = bmp_header + pixels + and_mask
ico_dir = struct.pack('<BBBBHHII',
    32,                 # width
    32,                 # height
    0,                  # colors
    0,                  # reserved
    1,                  # planes
    32,                 # bpp
    len(icon_data),     # size
    22                  # offset (6 + 16)
)

# ICO header
ico_header = struct.pack('<HHH', 0, 1, 1)  # reserved, type=icon, count=1

with open('icons/icon.ico', 'wb') as f:
    f.write(ico_header)
    f.write(ico_dir)
    f.write(icon_data)

print("Icon created successfully!")
