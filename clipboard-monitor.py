#!/usr/bin/env python3
"""
Clipboard Monitor - Real-time clipboard content viewer
Monitors clipboard and displays new content as it's copied
"""

import sys
import time
from datetime import datetime

try:
    import pyperclip
except ImportError:
    print("Error: pyperclip module not found")
    print("Install it with: pip install pyperclip")
    print("\nOn Linux you may also need:")
    print("  - xclip: sudo apt install xclip")
    print("  - or xsel: sudo apt install xsel")
    sys.exit(1)


def format_clipboard_entry(content, timestamp):
    """Format clipboard entry for display"""
    separator = "=" * 60
    print(f"\n{separator}")
    print(f"📋 Clipboard Update: {timestamp}")
    print(separator)

    # Show content type
    if content.strip() == "":
        print("⚠️  Empty/Whitespace content")
    elif content.startswith("http://") or content.startswith("https://"):
        print("🔗 URL detected")
    elif "\n" in content:
        print(f"📝 Multi-line text ({len(content.splitlines())} lines)")
    else:
        print("📄 Text")

    # Show content
    print("\nContent:")
    print("-" * 60)

    # Limit display length for very long content
    max_length = 1000
    if len(content) > max_length:
        print(content[:max_length])
        print(f"\n... (truncated, total length: {len(content)} characters)")
    else:
        print(content)

    print("-" * 60)


def monitor_clipboard(poll_interval=0.5):
    """Monitor clipboard for changes"""
    print("🚀 Clipboard Monitor Started")
    print("=" * 60)
    print("Monitoring clipboard in real-time...")
    print("Press Ctrl+C to stop")
    print("=" * 60)

    last_content = ""

    try:
        # Get initial clipboard content
        try:
            last_content = pyperclip.paste()
            if last_content:
                print("\n📌 Initial clipboard content:")
                format_clipboard_entry(last_content, datetime.now().strftime("%Y-%m-%d %H:%M:%S"))
        except Exception as e:
            print(f"⚠️  Could not read initial clipboard: {e}")

        # Monitor for changes
        while True:
            try:
                current_content = pyperclip.paste()

                # Check if content changed
                if current_content != last_content:
                    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                    format_clipboard_entry(current_content, timestamp)
                    last_content = current_content

            except Exception as e:
                print(f"\n⚠️  Error reading clipboard: {e}")

            time.sleep(poll_interval)

    except KeyboardInterrupt:
        print("\n\n" + "=" * 60)
        print("👋 Clipboard Monitor Stopped")
        print("=" * 60)
        sys.exit(0)


if __name__ == "__main__":
    monitor_clipboard()
