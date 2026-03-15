#!/usr/bin/env python3
"""
Generate a demo video showing dude in action.
Requires: pip install Pillow
Then: python generate_demo.py
Produces: demo.mp4
"""

import subprocess
import os
import struct
import wave
import math
import random
import shutil
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

# --- Config ---
WIDTH, HEIGHT = 1280, 720
FPS = 30
BG = (30, 30, 46)       # dark background
FG = (205, 214, 244)    # main text
YELLOW = (249, 226, 175) # dude: prompt
GREEN = (166, 227, 161)  # accepted/success
RED = (243, 139, 168)    # error
DIM = (108, 112, 134)    # dimmed text
CYAN = (137, 220, 235)   # suggestions
WHITE = (255, 255, 255)
PROMPT_COLOR = (137, 180, 250)  # blue prompt

# Terminal area limits
TERM_TOP = 65        # below title bar
TERM_BOTTOM = HEIGHT - 20
MAX_VISIBLE_LINES = 18  # max lines before scrolling

FRAME_DIR = Path("frames")
OUTPUT = "demo.mp4"
AUDIO_FILE = "bgm.wav"
TYPING_FILE = "typing.wav"
SAMPLE_RATE = 44100

# Try to find a monospace font
FONT_CANDIDATES = [
    "/System/Library/Fonts/SFMono-Regular.otf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Monaco.dfont",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
]

FONT_PATH = None
for f in FONT_CANDIDATES:
    if os.path.exists(f):
        FONT_PATH = f
        break

FONT_SIZE = 22
FONT_SIZE_BIG = 40
LINE_HEIGHT = 32


def get_font(size=FONT_SIZE):
    if FONT_PATH:
        return ImageFont.truetype(FONT_PATH, size)
    return ImageFont.load_default()


FONT = get_font(FONT_SIZE)
FONT_BIG = get_font(FONT_SIZE_BIG)


# ═══════════════════════════════════════════════════════════════════════════
# AUDIO
# ═══════════════════════════════════════════════════════════════════════════

def generate_typing_clicks(total_frames):
    """
    Generate a WAV file with click sounds at every frame that has typing.
    We'll generate enough silence for the full video and overlay clicks later.
    Returns a list of samples (mono, 16-bit) for the full duration.
    """
    sr = SAMPLE_RATE
    total_secs = total_frames / FPS
    total_samples = int(sr * total_secs)
    return [0] * total_samples


def make_click_samples(sr=SAMPLE_RATE):
    """Generate a single short mechanical click (~15ms)."""
    duration = 0.015
    n = int(sr * duration)
    samples = []
    for i in range(n):
        t = i / sr
        env = (1.0 - t / duration) ** 3  # sharp decay
        # Mix of high-freq components for a clicky feel
        val = (math.sin(2 * math.pi * 4500 * t) * 0.4
               + math.sin(2 * math.pi * 6800 * t) * 0.3
               + math.sin(2 * math.pi * 2200 * t) * 0.2)
        val *= env * 0.35
        samples.append(val)
    return samples


def generate_music(duration_secs):
    """
    Generate a fun lo-fi chiptune BGM.
    Pure Python — no dependencies beyond stdlib.
    Returns list of 16-bit sample ints.
    """
    sr = SAMPLE_RATE
    total_samples = int(sr * duration_secs)
    samples = []

    # Musical notes (Hz)
    NOTE = {
        "C4": 261.63, "D4": 293.66, "E4": 329.63, "F4": 349.23,
        "G4": 392.00, "A4": 440.00, "B4": 493.88,
        "C5": 523.25, "D5": 587.33, "E5": 659.25,
        "G3": 196.00, "A3": 220.00, "B3": 246.94,
        "F3": 174.61, "E3": 164.81, "D3": 146.83, "C3": 130.81,
        "R": 0,  # rest
    }

    # Fun bouncy melody — think lazy Sunday morning cartoon
    melody = [
        # Phrase 1 — "here comes dude"
        ("E4", 0.25), ("G4", 0.25), ("A4", 0.25), ("B4", 0.25),
        ("C5", 0.5),  ("B4", 0.25), ("A4", 0.25),
        ("G4", 0.5),  ("E4", 0.25), ("D4", 0.25),
        ("E4", 0.75), ("R", 0.25),
        # Phrase 2 — playful descend
        ("C5", 0.25), ("B4", 0.25), ("A4", 0.25), ("G4", 0.25),
        ("A4", 0.5),  ("G4", 0.25), ("E4", 0.25),
        ("D4", 0.5),  ("E4", 0.25), ("G4", 0.25),
        ("E4", 0.75), ("R", 0.25),
        # Phrase 3 — upbeat resolve
        ("G4", 0.25), ("A4", 0.25), ("B4", 0.25), ("C5", 0.25),
        ("D5", 0.5),  ("C5", 0.25), ("B4", 0.25),
        ("A4", 0.5),  ("G4", 0.25), ("A4", 0.25),
        ("G4", 0.75), ("R", 0.25),
        # Phrase 4 — chill ending
        ("E4", 0.25), ("D4", 0.25), ("C4", 0.25), ("D4", 0.25),
        ("E4", 0.5),  ("G4", 0.5),
        ("E4", 0.75), ("R", 0.25),
    ]

    # Bass line (plays under the melody, loops independently)
    bassline = [
        ("C3", 0.5), ("C3", 0.25), ("E3", 0.25),
        ("G3", 0.5), ("G3", 0.25), ("E3", 0.25),
        ("A3", 0.5), ("A3", 0.25), ("E3", 0.25),
        ("G3", 0.5), ("F3", 0.25), ("D3", 0.25),
    ]

    def square_wave(freq, t, duty=0.5):
        """Chiptune square wave with variable duty cycle."""
        if freq == 0:
            return 0.0
        phase = (t * freq) % 1.0
        return 0.3 if phase < duty else -0.3

    def triangle_wave(freq, t):
        """Softer triangle wave for bass."""
        if freq == 0:
            return 0.0
        phase = (t * freq) % 1.0
        return (4.0 * abs(phase - 0.5) - 1.0) * 0.25

    def noise_hat(t, beat_len=0.25):
        """Lo-fi hi-hat using deterministic noise."""
        beat_phase = (t % beat_len) / beat_len
        if beat_phase < 0.05:
            # Short burst of "noise" via high-freq sine mix
            val = (math.sin(t * 7919) * 0.5 + math.sin(t * 13999) * 0.3
                   + math.sin(t * 23456) * 0.2)
            return val * 0.08 * (1.0 - beat_phase / 0.05)
        return 0.0

    def kick(t, beat_len=0.5):
        """Simple synth kick drum."""
        beat_phase = (t % beat_len) / beat_len
        if beat_phase < 0.08:
            env = 1.0 - beat_phase / 0.08
            freq = 150 * (1.0 - beat_phase * 8)
            return math.sin(2 * math.pi * freq * t) * env * 0.3
        return 0.0

    # Pre-compute melody timeline
    melody_total = sum(d for _, d in melody)
    bass_total = sum(d for _, d in bassline)

    for i in range(total_samples):
        t = i / sr
        loop_t = t % melody_total  # melody loops

        # Find current melody note
        acc = 0.0
        mel_freq = 0
        for note_name, dur in melody:
            if acc + dur > loop_t:
                mel_freq = NOTE[note_name]
                break
            acc += dur

        # Find current bass note
        bass_loop_t = t % bass_total
        acc = 0.0
        bass_freq = 0
        for note_name, dur in bassline:
            if acc + dur > bass_loop_t:
                bass_freq = NOTE[note_name]
                break
            acc += dur

        # Mix
        mel = square_wave(mel_freq, t, duty=0.35)
        bas = triangle_wave(bass_freq, t)
        hat = noise_hat(t, 0.25)
        kck = kick(t, 0.5)

        # Simple envelope on melody to avoid harsh clicks
        mixed = mel * 0.5 + bas * 0.7 + hat + kck

        # Soft clip
        mixed = max(-0.9, min(0.9, mixed))

        # Volume — keep it chill background level
        mixed *= 0.4

        sample = int(mixed * 32767)
        samples.append(sample)

    return samples


def generate_audio(duration_secs, typing_frames):
    """
    Generate final audio: music + typing clicks overlaid.
    typing_frames is a set of frame numbers where a key was typed.
    """
    sr = SAMPLE_RATE
    total_samples = int(sr * duration_secs)

    # Generate music
    music = generate_music(duration_secs)

    # Generate click template
    click = make_click_samples(sr)
    click_len = len(click)

    # Overlay clicks onto music
    for frame_num in typing_frames:
        sample_pos = int(frame_num / FPS * sr)
        for j in range(click_len):
            idx = sample_pos + j
            if idx < total_samples:
                music[idx] = max(-32767, min(32767, music[idx] + int(click[j] * 32767)))

    # Write WAV
    with wave.open(AUDIO_FILE, "w") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sr)
        wf.writeframes(struct.pack(f"<{len(music)}h", *music))

    print(f"Generated {AUDIO_FILE} ({duration_secs:.1f}s, {len(typing_frames)} clicks)")


# ═══════════════════════════════════════════════════════════════════════════
# RENDERING
# ═══════════════════════════════════════════════════════════════════════════

# Global set to track which frames have typing (for click sounds)
typing_frame_set = set()


class Terminal:
    """Simulates a terminal screen that can be rendered to frames."""

    def __init__(self):
        self.lines = []
        self.cursor_visible = True

    def clear(self):
        self.lines = []

    def add_line(self, segments):
        """Add a line. Auto-scroll if exceeding visible area."""
        self.lines.append(segments)
        # Scroll: keep only the last MAX_VISIBLE_LINES
        if len(self.lines) > MAX_VISIBLE_LINES:
            self.lines = self.lines[-MAX_VISIBLE_LINES:]

    def render(self, typing_line=None, cursor_pos=None):
        """Render current state to a PIL Image."""
        img = Image.new("RGB", (WIDTH, HEIGHT), BG)
        draw = ImageDraw.Draw(img)

        x_start = 40

        # Draw window chrome
        draw.rounded_rectangle([15, 10, WIDTH - 15, 50], radius=8, fill=(49, 50, 68))
        draw.ellipse([25, 22, 39, 36], fill=(243, 139, 168))
        draw.ellipse([47, 22, 61, 36], fill=(249, 226, 175))
        draw.ellipse([69, 22, 83, 36], fill=(166, 227, 161))
        title_bar_font = get_font(16)
        draw.text((center_x("Terminal", title_bar_font), 18), "Terminal", fill=DIM, font=title_bar_font)

        y = TERM_TOP

        for line_segments in self.lines:
            if y + LINE_HEIGHT > TERM_BOTTOM:
                break
            x = x_start
            for text, color in line_segments:
                draw.text((x, y), text, fill=color, font=FONT)
                bbox = FONT.getbbox(text)
                x += bbox[2] - bbox[0]
            y += LINE_HEIGHT

        # Draw typing line with cursor
        if typing_line is not None and y + LINE_HEIGHT <= TERM_BOTTOM:
            x = x_start
            for text, color in typing_line:
                draw.text((x, y), text, fill=color, font=FONT)
                bbox = FONT.getbbox(text)
                x += bbox[2] - bbox[0]
            if cursor_pos is not None and self.cursor_visible:
                draw.rectangle([x, y, x + 12, y + FONT_SIZE + 2], fill=FG)

        return img


def text_width(text, font=None):
    f = font or FONT
    bbox = f.getbbox(text)
    return bbox[2] - bbox[0]


def center_x(text, font=None):
    return (WIDTH - text_width(text, font)) // 2


def save_frame(img, frame_num):
    img.save(FRAME_DIR / f"frame_{frame_num:06d}.png")


def hold_frames(term, count, frame_counter):
    """Hold current terminal state for N frames."""
    img = term.render()
    for _ in range(count):
        save_frame(img, frame_counter)
        frame_counter += 1
    return frame_counter


def type_text(term, prompt_segments, text, frame_counter, char_delay=2):
    """Animate typing character by character. Returns (frame_counter, final_text)."""
    for i in range(len(text) + 1):
        partial = text[:i]
        typing_line = prompt_segments + [(partial, FG)]
        img = term.render(typing_line=typing_line, cursor_pos=True)
        for d in range(char_delay):
            save_frame(img, frame_counter)
            # Mark the first frame of each new char as a typing frame
            if d == 0 and i > 0:
                typing_frame_set.add(frame_counter)
            frame_counter += 1
    # Commit the finished line to the terminal so there's no blink gap
    term.add_line(prompt_segments + [(text, FG)])
    return frame_counter


# ═══════════════════════════════════════════════════════════════════════════
# SCENES
# ═══════════════════════════════════════════════════════════════════════════

PROMPT = [("~/projects ", PROMPT_COLOR), ("\u276f ", GREEN)]

# ═══════════════════════════════════════════════════════════════════════════
# PIXEL ART HELPERS
# ═══════════════════════════════════════════════════════════════════════════

# Catppuccin-inspired pixel palette
SKY_TOP = (116, 199, 236)     # sky blue
SKY_BOT = (180, 227, 243)     # lighter horizon
SUN_COL = (249, 226, 175)     # warm yellow
SUN_GLOW = (250, 240, 210)
CLOUD_COL = (235, 240, 250)
GRASS = (166, 227, 161)       # green
GRASS_DARK = (130, 200, 130)
WALL_COL = (69, 71, 90)       # catppuccin surface2
WALL_LIGHT = (88, 91, 112)
FLOOR_COL = (49, 50, 68)      # darker
DESK_COL = (137, 115, 85)     # wood brown
DESK_DARK = (110, 90, 65)
MONITOR_BODY = (30, 30, 46)   # matches terminal BG
MONITOR_BEZEL = (49, 50, 68)
MONITOR_SCREEN_OFF = (40, 42, 54)
MONITOR_SCREEN_ON = (30, 30, 46)
CHAIR_COL = (69, 71, 90)
SKIN = (235, 200, 170)
SHIRT = (137, 180, 250)       # blue prompt color
PANTS = (69, 71, 90)
HAIR = (80, 60, 40)
WINDOW_FRAME = (88, 91, 112)
WINDOW_GLASS = (148, 210, 230)


def draw_pixel_rect(draw, x, y, w, h, color, scale=4):
    """Draw a scaled pixel rectangle."""
    draw.rectangle([x * scale, y * scale, (x + w) * scale - 1, (y + h) * scale - 1], fill=color)


def draw_scene_bg(draw, s=4):
    """Draw the room background: wall, floor, window, desk, monitor, chair."""
    W = WIDTH // s
    H = HEIGHT // s

    # Sky through window
    for row in range(H):
        t = row / H
        r = int(SKY_TOP[0] * (1 - t) + SKY_BOT[0] * t)
        g = int(SKY_TOP[1] * (1 - t) + SKY_BOT[1] * t)
        b = int(SKY_TOP[2] * (1 - t) + SKY_BOT[2] * t)
        draw_pixel_rect(draw, 0, row, W, 1, (r, g, b), s)

    # Wall (covers most of frame)
    draw_pixel_rect(draw, 0, 0, W, H, WALL_COL, s)

    # Floor
    floor_y = int(H * 0.72)
    draw_pixel_rect(draw, 0, floor_y, W, H - floor_y, FLOOR_COL, s)
    # Floor line
    draw_pixel_rect(draw, 0, floor_y, W, 1, WALL_LIGHT, s)

    # Window (left side)
    wx, wy, ww, wh = 20, 18, 50, 40
    draw_pixel_rect(draw, wx - 2, wy - 2, ww + 4, wh + 4, WINDOW_FRAME, s)
    # Sky gradient inside window
    for row in range(wh):
        t = row / wh
        r = int(SKY_TOP[0] * (1 - t) + SKY_BOT[0] * t)
        g = int(SKY_TOP[1] * (1 - t) + SKY_BOT[1] * t)
        b = int(SKY_TOP[2] * (1 - t) + SKY_BOT[2] * t)
        draw_pixel_rect(draw, wx, wy + row, ww, 1, (r, g, b), s)
    # Sun in window
    sun_cx, sun_cy = wx + 12, wy + 10
    for dy in range(-5, 6):
        for dx in range(-5, 6):
            dist = (dx * dx + dy * dy) ** 0.5
            if dist < 4:
                draw_pixel_rect(draw, sun_cx + dx, sun_cy + dy, 1, 1, SUN_COL, s)
            elif dist < 5.5:
                draw_pixel_rect(draw, sun_cx + dx, sun_cy + dy, 1, 1, SUN_GLOW, s)
    # Clouds
    for cx, cy in [(wx + 30, wy + 8), (wx + 38, wy + 14)]:
        for dx, dy in [(-3, 0), (-2, -1), (-1, -1), (0, -1), (1, -1), (2, 0),
                       (-2, 0), (-1, 0), (0, 0), (1, 0), (3, 0)]:
            draw_pixel_rect(draw, cx + dx, cy + dy, 1, 1, CLOUD_COL, s)
    # Window cross
    draw_pixel_rect(draw, wx + ww // 2, wy, 1, wh, WINDOW_FRAME, s)
    draw_pixel_rect(draw, wx, wy + wh // 2, ww, 1, WINDOW_FRAME, s)
    # Grass at bottom of window
    draw_pixel_rect(draw, wx, wy + wh - 5, ww, 5, GRASS, s)

    # Desk (right-center)
    dx, dy, dw, dh = 180, floor_y - 22, 90, 4
    draw_pixel_rect(draw, dx, dy, dw, dh, DESK_COL, s)         # desktop surface
    draw_pixel_rect(draw, dx, dy + dh, dw, 1, DESK_DARK, s)    # shadow
    # Desk legs
    draw_pixel_rect(draw, dx + 5, dy + dh, 3, floor_y - dy - dh, DESK_DARK, s)
    draw_pixel_rect(draw, dx + dw - 8, dy + dh, 3, floor_y - dy - dh, DESK_DARK, s)

    # Monitor on desk
    mx, my = dx + 30, dy - 30
    mw, mh = 32, 24
    # Monitor bezel
    draw_pixel_rect(draw, mx - 2, my - 2, mw + 4, mh + 4, MONITOR_BEZEL, s)
    # Screen (off)
    draw_pixel_rect(draw, mx, my, mw, mh, MONITOR_SCREEN_OFF, s)
    # Monitor stand
    draw_pixel_rect(draw, mx + mw // 2 - 3, my + mh + 2, 6, 6, MONITOR_BEZEL, s)
    draw_pixel_rect(draw, mx + mw // 2 - 6, my + mh + 7, 12, 2, MONITOR_BEZEL, s)

    # Chair (in front of desk)
    cx = dx + 35
    cy = floor_y - 16
    # Seat
    draw_pixel_rect(draw, cx - 8, cy, 16, 4, CHAIR_COL, s)
    # Back
    draw_pixel_rect(draw, cx - 8, cy - 14, 3, 14, CHAIR_COL, s)
    # Legs
    draw_pixel_rect(draw, cx - 6, cy + 4, 2, floor_y - cy - 4, CHAIR_COL, s)
    draw_pixel_rect(draw, cx + 4, cy + 4, 2, floor_y - cy - 4, CHAIR_COL, s)

    return {
        "desk_x": dx, "desk_y": dy, "desk_w": dw,
        "monitor_x": mx, "monitor_y": my, "monitor_w": mw, "monitor_h": mh,
        "chair_x": cx, "chair_y": cy,
        "floor_y": floor_y,
    }


def draw_character(draw, x, y, s=4, sitting=False):
    """Draw a simple pixel character at position (x, y=feet bottom)."""
    if sitting:
        # Sitting: shorter, legs bent
        # Head
        draw_pixel_rect(draw, x - 3, y - 22, 6, 6, SKIN, s)
        # Hair
        draw_pixel_rect(draw, x - 3, y - 23, 6, 2, HAIR, s)
        # Body
        draw_pixel_rect(draw, x - 3, y - 16, 6, 8, SHIRT, s)
        # Arms forward (typing)
        draw_pixel_rect(draw, x + 3, y - 14, 5, 2, SKIN, s)
        draw_pixel_rect(draw, x - 8, y - 14, 5, 2, SKIN, s)
        # Legs (bent)
        draw_pixel_rect(draw, x - 3, y - 8, 3, 5, PANTS, s)
        draw_pixel_rect(draw, x, y - 8, 3, 5, PANTS, s)
        # Feet
        draw_pixel_rect(draw, x - 4, y - 3, 3, 2, HAIR, s)
        draw_pixel_rect(draw, x + 1, y - 3, 3, 2, HAIR, s)
    else:
        # Standing / walking
        # Head
        draw_pixel_rect(draw, x - 3, y - 30, 6, 6, SKIN, s)
        # Hair
        draw_pixel_rect(draw, x - 3, y - 31, 6, 2, HAIR, s)
        # Body
        draw_pixel_rect(draw, x - 3, y - 24, 6, 10, SHIRT, s)
        # Arms
        draw_pixel_rect(draw, x - 5, y - 23, 2, 8, SKIN, s)
        draw_pixel_rect(draw, x + 3, y - 23, 2, 8, SKIN, s)
        # Legs
        draw_pixel_rect(draw, x - 3, y - 14, 3, 10, PANTS, s)
        draw_pixel_rect(draw, x, y - 14, 3, 10, PANTS, s)
        # Feet
        draw_pixel_rect(draw, x - 4, y - 4, 4, 3, HAIR, s)
        draw_pixel_rect(draw, x, y - 4, 4, 3, HAIR, s)


def draw_monitor_content(draw, mx, my, mw, mh, s, stage):
    """Draw monitor screen content based on boot stage."""
    if stage == "off":
        draw_pixel_rect(draw, mx, my, mw, mh, MONITOR_SCREEN_OFF, s)
    elif stage == "glow":
        # Screen turning on — slight glow
        draw_pixel_rect(draw, mx, my, mw, mh, (35, 36, 52), s)
    elif stage == "boot":
        # Boot screen
        draw_pixel_rect(draw, mx, my, mw, mh, MONITOR_SCREEN_ON, s)
        # Tiny text lines (simulated)
        for i in range(4):
            draw_pixel_rect(draw, mx + 2, my + 2 + i * 3, 12 - i * 2, 1, DIM, s)
    elif stage == "terminal":
        # Terminal ready
        draw_pixel_rect(draw, mx, my, mw, mh, MONITOR_SCREEN_ON, s)
        # Traffic lights
        draw_pixel_rect(draw, mx + 2, my + 2, 2, 2, RED, s)
        draw_pixel_rect(draw, mx + 5, my + 2, 2, 2, SUN_COL, s)
        draw_pixel_rect(draw, mx + 8, my + 2, 2, 2, GREEN, s)
        # Prompt line
        draw_pixel_rect(draw, mx + 2, my + 7, 8, 1, PROMPT_COLOR, s)
        draw_pixel_rect(draw, mx + 11, my + 7, 2, 1, GREEN, s)
    elif stage == "dude":
        # Terminal with dude loaded
        draw_pixel_rect(draw, mx, my, mw, mh, MONITOR_SCREEN_ON, s)
        # Traffic lights
        draw_pixel_rect(draw, mx + 2, my + 2, 2, 2, RED, s)
        draw_pixel_rect(draw, mx + 5, my + 2, 2, 2, SUN_COL, s)
        draw_pixel_rect(draw, mx + 8, my + 2, 2, 2, GREEN, s)
        # "dude" text in yellow
        draw_pixel_rect(draw, mx + 2, my + 7, 10, 1, YELLOW, s)
        # Prompt
        draw_pixel_rect(draw, mx + 2, my + 10, 8, 1, PROMPT_COLOR, s)
        draw_pixel_rect(draw, mx + 11, my + 10, 2, 1, GREEN, s)
        # Cursor blink
        draw_pixel_rect(draw, mx + 14, my + 10, 2, 1, FG, s)


def scene_intro(frame_counter):
    """
    Pixel-art cinematic intro (fast ~3s):
    Sunny day, guy walks to computer, boots it up, dude starts.
    """
    s = 4  # pixel scale

    # --- Phase 1: Empty room (0.3s) ---
    for f in range(FPS // 3):
        img = Image.new("RGB", (WIDTH, HEIGHT), WALL_COL)
        draw = ImageDraw.Draw(img)
        info = draw_scene_bg(draw, s)
        save_frame(img, frame_counter)
        frame_counter += 1

    # --- Phase 2: Character walks in from left to chair (1s) ---
    start_x = -10
    end_x = info["chair_x"]
    walk_frames = FPS
    for f in range(walk_frames):
        t = f / walk_frames
        t_ease = 1 - (1 - t) ** 2
        char_x = int(start_x + (end_x - start_x) * t_ease)

        img = Image.new("RGB", (WIDTH, HEIGHT), WALL_COL)
        draw = ImageDraw.Draw(img)
        info = draw_scene_bg(draw, s)
        draw_character(draw, char_x, info["floor_y"], s, sitting=False)
        save_frame(img, frame_counter)
        frame_counter += 1

    # --- Phase 3: Sit down (0.2s) ---
    for f in range(FPS // 5):
        img = Image.new("RGB", (WIDTH, HEIGHT), WALL_COL)
        draw = ImageDraw.Draw(img)
        info = draw_scene_bg(draw, s)
        draw_character(draw, info["chair_x"], info["chair_y"] + 4, s, sitting=True)
        save_frame(img, frame_counter)
        frame_counter += 1

    # --- Phase 4: Screen turns on fast (glow → boot → terminal → dude) ---
    stages = [
        ("glow", FPS // 5),
        ("boot", FPS // 3),
        ("terminal", FPS // 3),
        ("dude", FPS // 2),
    ]
    for stage, duration in stages:
        for f in range(duration):
            img = Image.new("RGB", (WIDTH, HEIGHT), WALL_COL)
            draw = ImageDraw.Draw(img)
            info = draw_scene_bg(draw, s)
            draw_monitor_content(
                draw,
                info["monitor_x"], info["monitor_y"],
                info["monitor_w"], info["monitor_h"],
                s, stage
            )
            draw_character(draw, info["chair_x"], info["chair_y"] + 4, s, sitting=True)
            save_frame(img, frame_counter)
            frame_counter += 1

    return frame_counter


def scene_title(frame_counter):
    """Opening title card (2s)."""
    title_font = get_font(64)
    subtitle_font = FONT_BIG
    face_font = get_font(36)

    img = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(img)
    draw.text((center_x("The Dude", title_font), HEIGHT // 2 - 100), "The Dude", fill=YELLOW, font=title_font)
    draw.text((center_x("(\u2310\u25a0_\u25a0)", face_font), HEIGHT // 2 - 20), "(\u2310\u25a0_\u25a0)", fill=CYAN, font=face_font)
    draw.text((center_x("for REALLY lazy people", subtitle_font), HEIGHT // 2 + 40), "for REALLY lazy people", fill=DIM, font=subtitle_font)
    for _ in range(FPS * 2):
        save_frame(img, frame_counter)
        frame_counter += 1

    return frame_counter


def scene_typo(frame_counter):
    """Scene 1: Classic typo correction (~4s)."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "gti stauts", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 4, frame_counter)

    term.add_line([("zsh: command not found: gti", RED)])
    frame_counter = hold_frames(term, FPS // 3, frame_counter)

    term.add_line([("dude: ", YELLOW), ("git status", WHITE)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("  run it? [Enter/n] ", DIM)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    # User hits enter — show git output
    term.add_line([])
    term.add_line([("On branch main", FG)])
    term.add_line([("Changes not staged for commit:", FG)])
    term.add_line([("  modified:   ", RED), ("src/main.rs", FG)])
    term.add_line([("  modified:   ", RED), ("README.md", FG)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    return frame_counter


def scene_question(frame_counter):
    """Scene 2: Natural language query (~3s)."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "kill whatever is on port 3000", frame_counter, char_delay=1)
    frame_counter = hold_frames(term, FPS // 4, frame_counter)

    term.add_line([("dude: ", YELLOW), ("lsof -ti:3000 | xargs kill -9", WHITE)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    term.add_line([("  run it? [Enter/n] ", DIM)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    return frame_counter


def scene_meaning_of_life(frame_counter):
    """Scene 3: dude meaning of life → echo 42."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, 'dude "meaning of life"', frame_counter, char_delay=3)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ('echo "42"', WHITE)])
    frame_counter = hold_frames(term, FPS * 3, frame_counter)

    return frame_counter


def scene_pipe(frame_counter):
    """Scene 3: Pipe mode — analyze output (~4s)."""
    term = Terminal()

    frame_counter = type_text(
        term, PROMPT, 'cat error.log | dude "why is this broken"', frame_counter, char_delay=1
    )
    frame_counter = hold_frames(term, FPS // 4, frame_counter)

    term.add_line([("dude:", YELLOW)])
    frame_counter = hold_frames(term, FPS // 4, frame_counter)

    term.add_line([("Connection refused to localhost:5432 \u2014 your", FG)])
    term.add_line([("PostgreSQL server isn't running. Try:", FG)])
    term.add_line([])
    term.add_line([("  brew services start postgresql", CYAN)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    return frame_counter


def scene_followup(frame_counter):
    """Scene 4: Follow-up questions (session memory) (~5s)."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "find files larger than 100mb", frame_counter, char_delay=1)
    frame_counter = hold_frames(term, FPS // 4, frame_counter)

    term.add_line([("dude: ", YELLOW), ("find . -size +100M -type f", WHITE)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    # Follow-up
    frame_counter = type_text(term, PROMPT, "now only in downloads", frame_counter, char_delay=1)
    frame_counter = hold_frames(term, FPS // 4, frame_counter)

    term.add_line([("                              ", FG),
                   ("\u2190 just type in natural language", DIM)])
    term.add_line([("dude: ", YELLOW), ("find ~/Downloads -size +100M -type f", WHITE)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    term.add_line([])
    term.add_line([("  ^ session memory \u2014 it remembers context", DIM)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    return frame_counter


def scene_learning(frame_counter):
    """Scene 5: Dude learns your typos — 3 accepts = instant (~4s)."""
    term = Terminal()

    term.add_line([("  \u2014 1st time \u2014", DIM)])
    term.add_line(PROMPT + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (asked LLM)", DIM)])
    term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    term.add_line([])
    term.add_line([("  \u2014 2nd time \u2014", DIM)])
    term.add_line(PROMPT + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (asked LLM)", DIM)])
    term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    term.add_line([])
    term.add_line([("  \u2014 3rd time \u2014", DIM)])
    term.add_line(PROMPT + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (instant \u26a1)", GREEN)])
    term.add_line([("  ^ no LLM needed \u2014 dude learned this one", DIM)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    return frame_counter


def scene_safety(frame_counter):
    """Scene 7: Safety modes overview (no rm -rf demo — just explain modes)."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "dude status", frame_counter, char_delay=3)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("provider: ", DIM), ("ollama", WHITE)])
    term.add_line([("dude: ", YELLOW), ("model: ", DIM), ("qwen2.5-coder:1.5b", WHITE)])
    term.add_line([("dude: ", YELLOW), ("safety: ", DIM),
                   ("confirm (always ask before running)", DIM)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    term.add_line([])
    term.add_line([("  Safety modes:", CYAN)])
    term.add_line([("  \u2022 ", DIM), ("confirm", GREEN),
                   (" \u2014 always asks before running (default)", DIM)])
    term.add_line([("  \u2022 ", DIM), ("auto   ", YELLOW),
                   (" \u2014 safe cmds run instantly, others ask", DIM)])
    term.add_line([("  \u2022 ", DIM), ("yolo   ", CYAN),
                   (" \u2014 never asks (destructive still blocked)", DIM)])
    frame_counter = hold_frames(term, FPS * 3, frame_counter)

    return frame_counter


def scene_provider_switch(frame_counter):
    """Scene 8: Switch providers and models."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "dude provider claude", frame_counter, char_delay=3)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("provider set to ", DIM), ("claude", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    frame_counter = type_text(term, PROMPT, "dude model claude-haiku-4-5-20251001", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("claude model set to ", DIM),
                   ("claude-haiku-4-5-20251001", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # Switch back
    frame_counter = type_text(term, PROMPT, "dude provider ollama", frame_counter, char_delay=3)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("provider set to ", DIM), ("ollama", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    return frame_counter


def scene_montage(frame_counter):
    """Rapid-fire best examples — typos + questions (~8s)."""
    examples = [
        # (input, output, is_typo)
        ("suod apt update",       "sudo apt update",           True),
        ("brwe install ffmpeg",   "brew install ffmpeg",       True),
        ("kucbectl get pods",     "kubectl get pods",          True),
        ("how to exit vim",      ":wq",                        True),
        ("whats my ip",          "curl -s ifconfig.me",        True),
        ("show me large files",   "find . -size +100M -type f", True),
    ]

    term = Terminal()
    for i, (inp, out, is_typo) in enumerate(examples):
        # New page every 3 items
        if i == 3:
            term = Terminal()
        if is_typo:
            term.add_line(PROMPT + [(inp, FG)])
            term.add_line([("dude: ", YELLOW), (out, WHITE)])
            term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
        else:
            term.add_line(PROMPT + [(inp, FG)])
            term.add_line([("dude: ", YELLOW), (out, WHITE)])
        term.add_line([])
        frame_counter = hold_frames(term, FPS, frame_counter)

    return frame_counter


def scene_dude_help(frame_counter):
    """Scene 11: Show the full help output."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "dude", frame_counter, char_delay=4)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude \u2014 your shell companion", YELLOW)])
    term.add_line([])
    term.add_line([("  dude learn", WHITE),          ("       analyze your shell history", DIM)])
    term.add_line([("  dude profile", WHITE),        ("     see what dude knows about you", DIM)])
    term.add_line([("  dude ask <q>", WHITE),        ("     ask dude for a command", DIM)])
    term.add_line([("  dude status", WHITE),         ("      check provider status", DIM)])
    term.add_line([("  dude context <q>", WHITE),    ("   show what goes to the LLM", DIM)])
    term.add_line([("  dude provider [n]", WHITE),   ("  set provider (ollama/claude)", DIM)])
    term.add_line([("  dude model [name]", WHITE),   ("  set the current model", DIM)])
    term.add_line([("  dude config", WHITE),         ("      interactive TUI settings", DIM)])
    term.add_line([("  dude clear", WHITE),          ("       clear conversation session", DIM)])
    term.add_line([])
    term.add_line([("  dude <question>", WHITE),     ("   no subcommand needed", DIM)])
    term.add_line([("  cmd | dude <q>", WHITE),      ("    pipe mode", DIM)])
    term.add_line([("  just type it", WHITE),         ("      natural language works too", DIM)])
    frame_counter = hold_frames(term, FPS * 4, frame_counter)

    return frame_counter


def scene_outro(frame_counter):
    """Closing card."""
    title_font = get_font(52)
    face_font = get_font(28)
    img = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(img)
    draw.text((center_x("The Dude", title_font), HEIGHT // 2 - 120),
              "The Dude", fill=YELLOW, font=title_font)
    draw.text((center_x("(\u2310\u25a0_\u25a0)", face_font), HEIGHT // 2 - 50),
              "(\u2310\u25a0_\u25a0)", fill=CYAN, font=face_font)
    draw.text((center_x("github.com/buzzer-re/dude"), HEIGHT // 2),
              "github.com/buzzer-re/dude", fill=CYAN, font=FONT)
    draw.text((center_x("pip install laziness not required"), HEIGHT // 2 + 40),
              "pip install laziness not required", fill=DIM, font=FONT)
    draw.text((center_x("ollama  |  claude  |  zsh  |  bash  |  fish"), HEIGHT // 2 + 80),
              "ollama  |  claude  |  zsh  |  bash  |  fish", fill=FG, font=FONT)

    for _ in range(FPS * 2):
        save_frame(img, frame_counter)
        frame_counter += 1

    return frame_counter


# ═══════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════

def main():
    if FRAME_DIR.exists():
        shutil.rmtree(FRAME_DIR)
    FRAME_DIR.mkdir()

    print("Generating frames...")
    fc = 0
    fc = scene_intro(fc)
    print(f"  Intro: {fc} frames")
    fc = scene_title(fc)
    print(f"  Title: {fc} frames")
    fc = scene_typo(fc)
    print(f"  Typo: {fc} frames")
    fc = scene_question(fc)
    print(f"  Question: {fc} frames")
    fc = scene_pipe(fc)
    print(f"  Pipe: {fc} frames")
    fc = scene_followup(fc)
    print(f"  Follow-up: {fc} frames")
    fc = scene_learning(fc)
    print(f"  Learning: {fc} frames")
    fc = scene_montage(fc)
    print(f"  Montage: {fc} frames")
    fc = scene_outro(fc)
    print(f"  Outro: {fc} frames")

    total_secs = fc / FPS
    print(f"\nTotal: {fc} frames ({total_secs:.1f}s at {FPS}fps)")

    # Generate audio (music + typing clicks)
    print("Generating audio (music + typing clicks)...")
    generate_audio(total_secs, typing_frame_set)

    print(f"Encoding {OUTPUT} with audio...")
    subprocess.run(
        [
            "ffmpeg", "-y",
            "-framerate", str(FPS),
            "-i", str(FRAME_DIR / "frame_%06d.png"),
            "-i", AUDIO_FILE,
            "-c:v", "libx264",
            "-c:a", "aac",
            "-b:a", "128k",
            "-pix_fmt", "yuv420p",
            "-preset", "slow",
            "-crf", "18",
            "-shortest",
            OUTPUT,
        ],
        check=True,
    )

    print(f"Done! \u2192 {OUTPUT}")

    # Clean up
    shutil.rmtree(FRAME_DIR)
    os.remove(AUDIO_FILE)
    print("Cleaned up temp files.")


if __name__ == "__main__":
    main()
