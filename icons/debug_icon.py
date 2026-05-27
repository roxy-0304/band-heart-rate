import struct

with open('icons/icon.ico', 'rb') as f:
    d = f.read()

# ICO header
h = struct.unpack('<HHH', d[0:6])
print('ICO header:', h)

# Entry
e = struct.unpack('<BBBBHHII', d[6:22])
print('Entry: width={} height={} colors={} planes={} bpp={} size={} offset={}'.format(e[0], e[1], e[2], e[4], e[5], e[6], e[7]))

# BMP header (40 bytes starting at offset 22)
# offset 22: header size (4)
# offset 26: width (4)
# offset 30: height (4) - this is the issue!
hs = struct.unpack('<I', d[22:26])[0]
w = struct.unpack('<I', d[26:30])[0]
hgt = struct.unpack('<I', d[30:34])[0]
print('BMP header size: {}, width: {}, height: {}'.format(hs, w, hgt))
