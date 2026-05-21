import os
import re

PROSE_DIR = r"C:\Users\mkibb\Documents\deep-meteor\architext-core\vespers\prose"
OUTPUT_DIR = r"C:\Users\mkibb\Documents\deep-meteor\architext-core\vespers\output"
FINAL_FILE = os.path.join(OUTPUT_DIR, "final.md")

def get_word_count(text):
    # Basic word count splitting by whitespace
    return len(re.findall(r'\b\w+\b', text))

def assemble():
    print("Initializing Vespers Novel Assembly...")
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    
    assembled_chapters = []
    total_words = 0
    chapter_metrics = []
    
    # Sort and read chapters
    for ch in range(1, 21):
        filename = f"ch{ch:02d}_prose.md"
        filepath = os.path.join(PROSE_DIR, filename)
        
        if not os.path.exists(filepath):
            print(f"Error: {filename} is missing! Assembly aborted.")
            return
            
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read().strip()
            
        # Clean potential duplicate H1 markers if needed, or leave them clean
        # Let's count words in this chapter
        words = get_word_count(content)
        total_words += words
        
        # Extract title from first line if it's an H1
        lines = content.split('\n')
        title = f"Chapter {ch}"
        if lines and lines[0].startswith('# '):
            title = lines[0].replace('# ', '').strip()
            
        chapter_metrics.append((ch, title, words))
        
        # Add page break decoration (HTML for PDF/Epub rendering)
        chapter_section = f"<!-- PAGE BREAK -->\n\n{content}"
        assembled_chapters.append(chapter_section)
        print(f"Loaded Chapter {ch:02d}: {title} ({words} words)")
        
    # Generate metadata block & telemetry summary
    telemetry = f"""# VESPERS
By Paradroid

## Metadata
* **Total Chapters**: 20
* **Total Word Count**: {total_words:,} words
* **Setting**: Outpost Colony Coda / Area 52, Vespers Gas Giant
* **Style**: Haptic Sensory POV (Deaf POV), Gritty Literary Cybernetic Science Fiction

## Table of Chapters
"""
    for ch, title, words in chapter_metrics:
        telemetry += f"* Chapter {ch:02d}: **{title}** ({words:,} words)\n"
        
    telemetry += "\n---\n\n"
    
    # Concatenate all chapters
    full_text = telemetry + "\n\n".join(assembled_chapters)
    
    with open(FINAL_FILE, "w", encoding="utf-8") as f:
        f.write(full_text)
        
    print(f"\nSuccess! Assembled novel written to: {FINAL_FILE}")
    print(f"Total Words Assembled: {total_words:,} words.")

if __name__ == "__main__":
    assemble()
