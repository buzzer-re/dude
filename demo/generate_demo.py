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
    Generate a fun upbeat chiptune BGM with typing clicks baked in.
    """
    sr = SAMPLE_RATE
    total_samples = int(sr * duration_secs)
    samples = []

    NOTE = {
        "C4": 261.63, "D4": 293.66, "E4": 329.63, "F4": 349.23,
        "G4": 392.00, "A4": 440.00, "B4": 493.88,
        "C5": 523.25, "D5": 587.33, "E5": 659.25, "F5": 698.46,
        "G3": 196.00, "A3": 220.00, "B3": 246.94,
        "F3": 174.61, "E3": 164.81, "D3": 146.83, "C3": 130.81,
        "R": 0,
    }

    # Bouncier, faster melody — more playful energy
    melody = [
        # Phrase 1 — bouncy opener
        ("E4", 0.15), ("R", 0.05), ("E4", 0.1), ("G4", 0.2),
        ("A4", 0.15), ("R", 0.05), ("A4", 0.1), ("B4", 0.2),
        ("C5", 0.3), ("B4", 0.15), ("A4", 0.15),
        ("G4", 0.3), ("R", 0.1),
        # Phrase 2 — playful bounce
        ("C5", 0.15), ("R", 0.05), ("C5", 0.1), ("D5", 0.2),
        ("E5", 0.3), ("D5", 0.15), ("C5", 0.15),
        ("B4", 0.2), ("A4", 0.1), ("G4", 0.2),
        ("A4", 0.3), ("R", 0.1),
        # Phrase 3 — ascending fun
        ("E4", 0.1), ("F4", 0.1), ("G4", 0.1), ("A4", 0.1),
        ("B4", 0.15), ("R", 0.05), ("B4", 0.1), ("C5", 0.2),
        ("D5", 0.3), ("C5", 0.15), ("B4", 0.15),
        ("A4", 0.3), ("R", 0.1),
        # Phrase 4 — chill resolve
        ("G4", 0.2), ("E4", 0.1), ("G4", 0.2),
        ("A4", 0.15), ("G4", 0.15), ("E4", 0.3),
        ("D4", 0.2), ("E4", 0.4), ("R", 0.2),
    ]

    bassline = [
        ("C3", 0.3), ("C3", 0.15), ("E3", 0.15),
        ("G3", 0.3), ("G3", 0.15), ("E3", 0.15),
        ("A3", 0.3), ("A3", 0.15), ("E3", 0.15),
        ("G3", 0.3), ("F3", 0.15), ("D3", 0.15),
    ]

    # Arp layer for extra sparkle
    arp = [
        ("E5", 0.1), ("G4", 0.1), ("C5", 0.1), ("R", 0.1),
        ("D5", 0.1), ("B4", 0.1), ("G4", 0.1), ("R", 0.1),
        ("C5", 0.1), ("A4", 0.1), ("E4", 0.1), ("R", 0.1),
    ]

    def square_wave(freq, t, duty=0.5):
        if freq == 0:
            return 0.0
        phase = (t * freq) % 1.0
        return 0.3 if phase < duty else -0.3

    def triangle_wave(freq, t):
        if freq == 0:
            return 0.0
        phase = (t * freq) % 1.0
        return (4.0 * abs(phase - 0.5) - 1.0) * 0.25

    def pulse_wave(freq, t, duty=0.25):
        """Thin pulse for arp — very chiptune."""
        if freq == 0:
            return 0.0
        phase = (t * freq) % 1.0
        return 0.15 if phase < duty else -0.15

    def noise_hat(t, beat_len=0.15):
        beat_phase = (t % beat_len) / beat_len
        if beat_phase < 0.04:
            val = (math.sin(t * 7919) * 0.5 + math.sin(t * 13999) * 0.3
                   + math.sin(t * 23456) * 0.2)
            return val * 0.06 * (1.0 - beat_phase / 0.04)
        return 0.0

    def kick(t, beat_len=0.3):
        beat_phase = (t % beat_len) / beat_len
        if beat_phase < 0.06:
            env = 1.0 - beat_phase / 0.06
            freq = 150 * (1.0 - beat_phase * 10)
            return math.sin(2 * math.pi * freq * t) * env * 0.25
        return 0.0

    melody_total = sum(d for _, d in melody)
    bass_total = sum(d for _, d in bassline)
    arp_total = sum(d for _, d in arp)

    def find_note(sequence, seq_total, t):
        loop_t = t % seq_total
        acc = 0.0
        for note_name, dur in sequence:
            if acc + dur > loop_t:
                return NOTE[note_name]
            acc += dur
        return 0

    for i in range(total_samples):
        t = i / sr

        mel_freq = find_note(melody, melody_total, t)
        bass_freq = find_note(bassline, bass_total, t)
        arp_freq = find_note(arp, arp_total, t)

        mel = square_wave(mel_freq, t, duty=0.35)
        bas = triangle_wave(bass_freq, t)
        arpv = pulse_wave(arp_freq, t, duty=0.25)
        hat = noise_hat(t, 0.15)
        kck = kick(t, 0.3)

        mixed = mel * 0.4 + bas * 0.6 + arpv * 0.25 + hat + kck

        # Soft clip
        mixed = max(-0.9, min(0.9, mixed))
        mixed *= 0.35

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


def scene_title(frame_counter):
    """Opening title card."""
    term = Terminal()
    frame_counter = hold_frames(term, FPS, frame_counter)

    title_font = get_font(64)
    subtitle_font = FONT_BIG
    face_font = get_font(36)

    img = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(img)
    draw.text((center_x("The Dude", title_font), HEIGHT // 2 - 100), "The Dude", fill=YELLOW, font=title_font)
    draw.text((center_x("(\u2310\u25a0_\u25a0)", face_font), HEIGHT // 2 - 20), "(\u2310\u25a0_\u25a0)", fill=CYAN, font=face_font)
    draw.text((center_x("for REALLY lazy people", subtitle_font), HEIGHT // 2 + 40), "for REALLY lazy people", fill=DIM, font=subtitle_font)
    for _ in range(FPS * 3):
        save_frame(img, frame_counter)
        frame_counter += 1

    return frame_counter


def scene_typo(frame_counter):
    """Scene 1: Classic typo correction."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "gti stauts", frame_counter, char_delay=3)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("zsh: command not found: gti", RED)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("git status", WHITE)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    term.add_line([("  run it? [Enter/n] ", DIM)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    # User hits enter — show git output
    term.add_line([])
    term.add_line([("On branch main", FG)])
    term.add_line([("Changes not staged for commit:", FG)])
    term.add_line([("  modified:   ", RED), ("src/main.rs", FG)])
    term.add_line([("  modified:   ", RED), ("README.md", FG)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    frame_counter = hold_frames(term, FPS, frame_counter)
    return frame_counter


def scene_question(frame_counter):
    """Scene 2: Ask dude a question with ? prefix."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "? kill whatever is on port 3000", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("lsof -ti:3000 | xargs kill -9", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    term.add_line([("  run it? [Enter/n] ", DIM)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

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
    """Scene 4: Pipe mode — analyze output."""
    term = Terminal()

    frame_counter = type_text(
        term, PROMPT, 'cat error.log | dude "why is this broken"', frame_counter, char_delay=2
    )
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude:", YELLOW)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("Connection refused to localhost:5432 \u2014 your", FG)])
    term.add_line([("PostgreSQL server isn't running. Try:", FG)])
    term.add_line([])
    term.add_line([("  brew services start postgresql", CYAN)])
    frame_counter = hold_frames(term, FPS * 3, frame_counter)

    return frame_counter


def scene_followup(frame_counter):
    """Scene 5: Follow-up questions (session memory)."""
    term = Terminal()

    frame_counter = type_text(term, PROMPT, "? find files larger than 100mb", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("find . -size +100M -type f", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # Follow-up
    frame_counter = type_text(term, PROMPT, "? now only in downloads", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("dude: ", YELLOW), ("find ~/Downloads -size +100M -type f", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    term.add_line([])
    term.add_line([("  ^ 15-min session memory \u2014 it knows what you asked", DIM)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    return frame_counter


def scene_learning(frame_counter):
    """Scene 6: Dude learns your typos — 3 accepts = instant forever."""
    term = Terminal()

    term.add_line([("  \u2014 1st time \u2014", DIM)])
    term.add_line(PROMPT + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (asked LLM)", DIM)])
    term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    term.add_line([])
    term.add_line([("  \u2014 2nd time \u2014", DIM)])
    term.add_line(PROMPT + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (asked LLM)", DIM)])
    term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    term.add_line([])
    term.add_line([("  \u2014 3rd time \u2014", DIM)])
    term.add_line(PROMPT + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (local DB, instant \u26a1)", GREEN)])
    term.add_line([("  ^ no LLM needed \u2014 dude learned this one", DIM)])
    frame_counter = hold_frames(term, FPS * 3, frame_counter)

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


def scene_funny_typos(frame_counter):
    """Scene 9: Funny typo corrections — split into pages to avoid overflow."""
    typos = [
        ("suod apt update",        "sudo apt update"),
        ("brwe install ffmpeg",    "brew install ffmpeg"),
        ("pytohn -m venv .venv",   "python -m venv .venv"),
        ("doker compose up -d",    "docker compose up -d"),
        ("kucbectl get pods",      "kubectl get pods"),
        ("sl",                     "ls"),
    ]

    # Page 1: first 3
    term = Terminal()
    for wrong, right in typos[:3]:
        term.add_line(PROMPT + [(wrong, FG)])
        term.add_line([("dude: ", YELLOW), (right, WHITE)])
        term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
        term.add_line([])
        frame_counter = hold_frames(term, int(FPS * 1.3), frame_counter)

    # Page 2: last 3
    term = Terminal()
    for wrong, right in typos[3:]:
        term.add_line(PROMPT + [(wrong, FG)])
        term.add_line([("dude: ", YELLOW), (right, WHITE)])
        term.add_line([("  run it? [Enter/n] ", DIM), ("\u2713", GREEN)])
        term.add_line([])
        frame_counter = hold_frames(term, int(FPS * 1.3), frame_counter)

    return frame_counter


def scene_funny_montage(frame_counter):
    """Scene 10: Rapid-fire lazy & funny queries — split into pages."""
    queries = [
        ("? how to exit vim",                              ":wq"),
        ("? undo last git commit but keep files",          "git reset --soft HEAD~1"),
        ("? whats my ip",                                  "curl -s ifconfig.me"),
        ("? disk space whats eating it",                   "du -sh * | sort -rh | head -20"),
        ("? am i connected to the internet or losing it",  "ping -c 3 8.8.8.8"),
        ("? mass rename .jpeg to .jpg",                    'for f in *.jpeg; do mv "$f" "${f%.jpeg}.jpg"; done'),
    ]

    # Page 1
    term = Terminal()
    for query, response in queries[:3]:
        term.add_line(PROMPT + [(query, FG)])
        term.add_line([("dude: ", YELLOW), ("  " + response, WHITE)])
        term.add_line([])
        frame_counter = hold_frames(term, int(FPS * 1.5), frame_counter)

    # Page 2
    term = Terminal()
    for query, response in queries[3:]:
        term.add_line(PROMPT + [(query, FG)])
        term.add_line([("dude: ", YELLOW), ("  " + response, WHITE)])
        term.add_line([])
        frame_counter = hold_frames(term, int(FPS * 1.5), frame_counter)

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
    term.add_line([("  ? <question>", WHITE),        ("       quick ask (shell plugin)", DIM)])
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

    for _ in range(FPS * 3):
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
    fc = scene_title(fc)
    print(f"  Title: {fc} frames")
    fc = scene_typo(fc)
    print(f"  Typo scene: {fc} frames")
    fc = scene_question(fc)
    print(f"  Question scene: {fc} frames")
    fc = scene_meaning_of_life(fc)
    print(f"  Meaning of life: {fc} frames")
    fc = scene_pipe(fc)
    print(f"  Pipe scene: {fc} frames")
    fc = scene_followup(fc)
    print(f"  Follow-up scene: {fc} frames")
    fc = scene_learning(fc)
    print(f"  Learning scene: {fc} frames")
    fc = scene_safety(fc)
    print(f"  Safety/status scene: {fc} frames")
    fc = scene_provider_switch(fc)
    print(f"  Provider switch scene: {fc} frames")
    fc = scene_funny_typos(fc)
    print(f"  Funny typos: {fc} frames")
    fc = scene_funny_montage(fc)
    print(f"  Funny montage: {fc} frames")
    fc = scene_dude_help(fc)
    print(f"  Help scene: {fc} frames")
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
