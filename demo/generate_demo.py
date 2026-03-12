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

FRAME_DIR = Path("frames")
OUTPUT = "demo.mp4"
AUDIO_FILE = "bgm.wav"
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


def generate_music(duration_secs):
    """
    Generate a fun lo-fi chiptune BGM.
    Pure Python — no dependencies beyond stdlib.
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
                # Envelope: slight attack + decay
                note_phase = (loop_t - acc) / dur
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

    # Write WAV
    with wave.open(AUDIO_FILE, "w") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sr)
        wf.writeframes(struct.pack(f"<{len(samples)}h", *samples))

    print(f"Generated {AUDIO_FILE} ({duration_secs:.1f}s)")


class Terminal:
    """Simulates a terminal screen that can be rendered to frames."""

    def __init__(self):
        self.lines = []  # list of (text, color) tuples per line
        self.cursor_visible = True

    def clear(self):
        self.lines = []

    def add_line(self, segments):
        """Add a line as list of (text, color) tuples."""
        self.lines.append(segments)

    def render(self, typing_line=None, cursor_pos=None):
        """Render current state to a PIL Image."""
        img = Image.new("RGB", (WIDTH, HEIGHT), BG)
        draw = ImageDraw.Draw(img)

        x_start = 40
        y = 30

        # Draw window chrome (fake title bar)
        draw.rounded_rectangle([15, 10, WIDTH - 15, 50], radius=8, fill=(49, 50, 68))
        # Traffic lights
        draw.ellipse([25, 22, 39, 36], fill=(243, 139, 168))
        draw.ellipse([47, 22, 61, 36], fill=(249, 226, 175))
        draw.ellipse([69, 22, 83, 36], fill=(166, 227, 161))
        title_bar_font = get_font(16)
        draw.text((center_x("Terminal", title_bar_font), 18), "Terminal", fill=DIM, font=title_bar_font)

        y = 65

        for line_segments in self.lines:
            x = x_start
            for text, color in line_segments:
                draw.text((x, y), text, fill=color, font=FONT)
                bbox = FONT.getbbox(text)
                x += bbox[2] - bbox[0]
            y += LINE_HEIGHT

        # Draw typing line with cursor
        if typing_line is not None:
            x = x_start
            for text, color in typing_line:
                draw.text((x, y), text, fill=color, font=FONT)
                bbox = FONT.getbbox(text)
                x += bbox[2] - bbox[0]
            if cursor_pos is not None and self.cursor_visible:
                draw.rectangle([x, y, x + 12, y + FONT_SIZE + 2], fill=FG)

        return img


def text_width(text, font=None):
    """Get pixel width of text string."""
    f = font or FONT
    bbox = f.getbbox(text)
    return bbox[2] - bbox[0]


def text_height(text, font=None):
    """Get pixel height of text string."""
    f = font or FONT
    bbox = f.getbbox(text)
    return bbox[3] - bbox[1]


def center_x(text, font=None):
    """Return x position to center text horizontally."""
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
    """Animate typing character by character."""
    for i in range(len(text) + 1):
        partial = text[:i]
        typing_line = prompt_segments + [(partial, FG)]
        img = term.render(typing_line=typing_line, cursor_pos=True)
        for _ in range(char_delay):
            save_frame(img, frame_counter)
            frame_counter += 1
    return frame_counter


def scene_title(frame_counter):
    """Opening title card."""
    term = Terminal()
    # Blank for a beat
    frame_counter = hold_frames(term, FPS, frame_counter)

    title_font = get_font(64)
    subtitle_font = FONT_BIG

    img = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(img)
    draw.text((center_x("The Dude", title_font), HEIGHT // 2 - 80), "The Dude", fill=YELLOW, font=title_font)
    draw.text((center_x("for REALLY lazy people", subtitle_font), HEIGHT // 2 + 10), "for REALLY lazy people", fill=DIM, font=subtitle_font)
    for _ in range(FPS * 3):
        save_frame(img, frame_counter)
        frame_counter += 1

    return frame_counter


def scene_typo(frame_counter):
    """Scene 1: Classic typo correction."""
    term = Terminal()
    prompt = [("~/projects ", PROMPT_COLOR), ("❯ ", GREEN)]

    # Type the wrong command
    frame_counter = type_text(term, prompt, "gti stauts", frame_counter, char_delay=3)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    # Press enter — show command on terminal
    term.add_line(prompt + [("gti stauts", FG)])
    term.add_line([("zsh: command not found: gti", RED)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    # Dude responds
    term.add_line([("dude: ", YELLOW), ("git status", WHITE)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    term.add_line([("  run it? [Enter/n] ", DIM)])
    frame_counter = hold_frames(term, FPS, frame_counter)

    # User hits enter
    term.add_line([])
    term.add_line([("On branch main", FG)])
    term.add_line([("Changes not staged for commit:", FG)])
    term.add_line([("  modified:   ", RED), ("src/main.rs", FG)])
    term.add_line([("  modified:   ", RED), ("README.md", FG)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # Pause
    frame_counter = hold_frames(term, FPS, frame_counter)
    return frame_counter


def scene_question(frame_counter):
    """Scene 2: Ask dude a question with ? prefix."""
    term = Terminal()
    prompt = [("~/projects ", PROMPT_COLOR), ("❯ ", GREEN)]

    # Type question
    frame_counter = type_text(term, prompt, "? kill whatever is on port 3000", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    # Response
    term.add_line(prompt + [("? kill whatever is on port 3000", FG)])
    term.add_line([("dude: ", YELLOW), ("lsof -ti:3000 | xargs kill -9", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    term.add_line([("  run it? [Enter/n] ", DIM)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    return frame_counter


def scene_pipe(frame_counter):
    """Scene 3: Pipe mode — analyze output."""
    term = Terminal()
    prompt = [("~/projects ", PROMPT_COLOR), ("❯ ", GREEN)]

    frame_counter = type_text(
        term, prompt, 'cat error.log | dude "why is this broken"', frame_counter, char_delay=2
    )
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line(prompt + [('cat error.log | dude "why is this broken"', FG)])
    term.add_line([("dude:", YELLOW)])
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line([("Connection refused to localhost:5432 — your", FG)])
    term.add_line([("PostgreSQL server isn't running. Try:", FG)])
    term.add_line([])
    term.add_line([("  brew services start postgresql", CYAN)])
    frame_counter = hold_frames(term, FPS * 3, frame_counter)

    return frame_counter


def scene_followup(frame_counter):
    """Scene 4: Follow-up questions (session memory)."""
    term = Terminal()
    prompt = [("~/projects ", PROMPT_COLOR), ("❯ ", GREEN)]

    # First question
    frame_counter = type_text(term, prompt, "? find files larger than 100mb", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line(prompt + [("? find files larger than 100mb", FG)])
    term.add_line([("dude: ", YELLOW), ("find . -size +100M -type f", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # Follow-up — dude remembers context!
    frame_counter = type_text(term, prompt, "? now only in downloads", frame_counter, char_delay=2)
    frame_counter = hold_frames(term, FPS // 2, frame_counter)

    term.add_line(prompt + [("? now only in downloads", FG)])
    term.add_line([("dude: ", YELLOW), ("find ~/Downloads -size +100M -type f", WHITE)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # Annotation
    term.add_line([])
    term.add_line([("  ^ it remembered what you asked 5 seconds ago", DIM)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    return frame_counter


def scene_learning(frame_counter):
    """Scene 5: Dude learns your typos — 3 accepts = instant forever."""
    term = Terminal()
    prompt = [("~/projects ", PROMPT_COLOR), ("❯ ", GREEN)]

    # 1st time — asks the LLM
    term.add_line([("  — 1st time —", DIM)])
    term.add_line(prompt + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (asked LLM)", DIM)])
    term.add_line([("  run it? [Enter/n] ", DIM), ("✓", GREEN)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # 2nd time — still asks LLM
    term.add_line([])
    term.add_line([("  — 2nd time —", DIM)])
    term.add_line(prompt + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (asked LLM)", DIM)])
    term.add_line([("  run it? [Enter/n] ", DIM), ("✓", GREEN)])
    frame_counter = hold_frames(term, FPS * 2, frame_counter)

    # 3rd time — accepts again, triggers learning
    term.add_line([])
    term.add_line([("  — 3rd time —", DIM)])
    term.add_line(prompt + [("dockr ps", FG)])
    term.add_line([("dude: ", YELLOW), ("docker ps", WHITE), ("  (local DB, instant ⚡)", GREEN)])
    term.add_line([("  ^ no LLM needed — dude learned this one", DIM)])
    frame_counter = hold_frames(term, FPS * 3, frame_counter)

    return frame_counter


def scene_lazy_montage(frame_counter):
    """Scene 6: Rapid-fire lazy person queries."""
    term = Terminal()
    prompt = [("~/projects ", PROMPT_COLOR), ("❯ ", GREEN)]

    lazy_queries = [
        ("? how to exit vim",                   "  :wq"),
        ("? undo last git commit but keep files","  git reset --soft HEAD~1"),
        ("? compress this folder for email",     "  tar czf folder.tar.gz folder/"),
        ("? whats my ip",                        "  curl -s ifconfig.me"),
        ("? disk space whats eating it",         "  du -sh * | sort -rh | head -20"),
    ]

    for query, response in lazy_queries:
        term.add_line(prompt + [(query, FG)])
        term.add_line([("dude: ", YELLOW), (response, WHITE)])
        term.add_line([])
        frame_counter = hold_frames(term, int(FPS * 1.5), frame_counter)

    return frame_counter


def scene_outro(frame_counter):
    """Closing card."""
    title_font = get_font(52)
    img = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(img)
    draw.text((center_x("The Dude", title_font), HEIGHT // 2 - 100), "The Dude", fill=YELLOW, font=title_font)
    draw.text((center_x("github.com/yourusername/dude"), HEIGHT // 2 - 20), "github.com/yourusername/dude", fill=CYAN, font=FONT)
    draw.text((center_x("pip install laziness not required"), HEIGHT // 2 + 30), "pip install laziness not required", fill=DIM, font=FONT)
    draw.text((center_x("ollama  |  claude"), HEIGHT // 2 + 80), "ollama  |  claude", fill=FG, font=FONT)

    for _ in range(FPS * 3):
        save_frame(img, frame_counter)
        frame_counter += 1

    return frame_counter


def main():
    # Clean up
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
    fc = scene_pipe(fc)
    print(f"  Pipe scene: {fc} frames")
    fc = scene_followup(fc)
    print(f"  Follow-up scene: {fc} frames")
    fc = scene_learning(fc)
    print(f"  Learning scene: {fc} frames")
    fc = scene_lazy_montage(fc)
    print(f"  Lazy montage: {fc} frames")
    fc = scene_outro(fc)
    print(f"  Outro: {fc} frames")

    total_secs = fc / FPS
    print(f"\nTotal: {fc} frames ({total_secs:.1f}s at {FPS}fps)")

    # Generate chiptune background music
    print("Generating background music...")
    generate_music(total_secs)

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

    print(f"Done! → {OUTPUT}")

    # Clean up
    shutil.rmtree(FRAME_DIR)
    os.remove(AUDIO_FILE)
    print("Cleaned up temp files.")


if __name__ == "__main__":
    main()
