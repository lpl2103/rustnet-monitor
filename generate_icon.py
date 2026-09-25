import math
from PIL import Image, ImageDraw, ImageFilter

def create_icon():
    # Render at 1024x1024 with supersampling, downsampling to multi-res ICO
    size = 1024
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    center = size / 2
    radius = size * 0.44

    # 1. Outer Glow
    glow_layers = 15
    for i in range(glow_layers):
        r = radius + (glow_layers - i) * 3
        alpha = int(12 * (1 - i / glow_layers))
        draw.ellipse([center - r, center - r, center + r, center + r],
                     fill=(0, 180, 255, alpha))

    # 2. Main Circular / Squircle Badge
    # Draw background circle with radial gradient
    bg_img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    bg_draw = ImageDraw.Draw(bg_img)
    
    # Gradient simulation from top-left (navy blue) to bottom-right (deep dark blue)
    steps = 100
    for s in range(steps, 0, -1):
        r = radius * (s / steps)
        t = 1.0 - (s / steps)
        # Interpolate color: (20, 32, 54) -> (10, 16, 28)
        red = int(22 - 12 * t)
        green = int(36 - 20 * t)
        blue = int(60 - 32 * t)
        bg_draw.ellipse([center - r, center - r, center + r, center + r],
                        fill=(red, green, blue, 255))
    
    img = Image.alpha_composite(img, bg_img)
    draw = ImageDraw.Draw(img)

    # 3. Outer Glowing Ring / Border
    ring_width = 18
    draw.ellipse([center - radius, center - radius, center + radius, center + radius],
                 outline=(0, 229, 255, 230), width=ring_width)
    
    # Concentric faint radar rings
    draw.ellipse([center - radius*0.72, center - radius*0.72, center + radius*0.72, center + radius*0.72],
                 outline=(0, 150, 255, 45), width=6)
    draw.ellipse([center - radius*0.44, center - radius*0.44, center + radius*0.44, center + radius*0.44],
                 outline=(0, 150, 255, 60), width=6)

    # Crosshair / Radar angle lines (subtle)
    draw.line([(center, center - radius*0.85), (center, center + radius*0.85)],
              fill=(0, 180, 255, 40), width=4)
    draw.line([(center - radius*0.85, center), (center + radius*0.85, center)],
              fill=(0, 180, 255, 40), width=4)

    # 4. Network Mesh / Graph Lines
    nodes = [
        (center - 240, center - 120),
        (center - 130, center - 220),
        (center + 130, center - 220),
        (center + 240, center - 120),
        (center - 200, center + 180),
        (center,       center + 250),
        (center + 200, center + 180),
        (center,       center), # Central node
    ]

    connections = [
        (0, 1), (1, 2), (2, 3),
        (0, 7), (1, 7), (2, 7), (3, 7),
        (4, 5), (5, 6),
        (4, 7), (5, 7), (6, 7),
        (0, 4), (3, 6)
    ]

    for start_idx, end_idx in connections:
        draw.line([nodes[start_idx], nodes[end_idx]], fill=(30, 140, 240, 90), width=6)

    # 5. Dynamic Pulse / Latency ECG Waveform (Horizontal across center)
    # Highlight showing real-time network connectivity
    pulse_points = [
        (center - radius * 0.82, center + 40),
        (center - 220, center + 40),
        (center - 160, center + 40),
        (center - 110, center - 130), # Sharp peak up
        (center - 50,  center + 160), # Valley down
        (center,       center - 80),  # Mid bounce
        (center + 50,  center + 40),  # Baseline
        (center + 100, center - 40),  # Second wave
        (center + 150, center + 40),
        (center + radius * 0.82, center + 40),
    ]

    # Draw glow for pulse line
    glow_pulse = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    glow_pulse_draw = ImageDraw.Draw(glow_pulse)
    for width, alpha in [(32, 35), (20, 70), (12, 120)]:
        glow_pulse_draw.line(pulse_points, fill=(0, 245, 255, alpha), width=width, joint="curve")
    img = Image.alpha_composite(img, glow_pulse)
    draw = ImageDraw.Draw(img)

    # Draw sharp pulse line (Neon Cyan / Electric White core)
    draw.line(pulse_points, fill=(230, 255, 255, 255), width=10, joint="curve")

    # 6. Nodes (Circles with glow)
    for idx, (nx, ny) in enumerate(nodes):
        # Peripheral node
        nr = 24 if idx == 7 else 18
        # Outer glow
        draw.ellipse([nx - nr - 8, ny - nr - 8, nx + nr + 8, ny + nr + 8],
                     fill=(0, 229, 255, 60))
        # Base circle
        draw.ellipse([nx - nr, ny - nr, nx + nr, ny + nr],
                     fill=(0, 200, 255, 240), outline=(255, 255, 255, 255), width=5)
        # Inner white dot
        draw.ellipse([nx - 6, ny - 6, nx + 6, ny + 6], fill=(255, 255, 255, 255))

    # Center pulse node (bright emerald / gold active beacon)
    cx, cy = center, center - 80 # Peak point on the pulse
    draw.ellipse([cx - 30, cy - 30, cx + 30, cy + 30], fill=(0, 255, 170, 70))
    draw.ellipse([cx - 18, cy - 18, cx + 18, cy + 18], fill=(0, 255, 170, 255),
                 outline=(255, 255, 255, 255), width=5)

    # Save high-res PNG
    png_256 = img.resize((256, 256), Image.Resampling.LANCZOS)
    png_256.save("assets/app.png", format="PNG")
    print("Saved assets/app.png (256x256)")

    # Save multi-resolution ICO (256, 128, 64, 48, 32, 24, 16)
    sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (24, 24), (16, 16)]
    icon_images = [img.resize(s, Image.Resampling.LANCZOS) for s in sizes]
    
    # Pillow save ICO with sizes
    icon_images[0].save(
        "assets/app.ico",
        format="ICO",
        sizes=sizes,
        append_images=icon_images[1:]
    )
    print("Saved assets/app.ico with resolutions:", sizes)

if __name__ == "__main__":
    create_icon()
