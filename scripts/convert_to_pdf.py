import sys
import os
import re
from fpdf import FPDF

class MarkdownPDF(FPDF):
    def header(self):
        if self.page_no() > 1:
            self.set_font("Helvetica", "I", 8)
            self.set_text_color(100, 116, 139) # Slate 500
            self.cell(0, 10, "Vespers Session Completion - Technical Archive", align="R")
            self.ln(10)

    def footer(self):
        self.set_y(-15)
        self.set_font("Helvetica", "I", 8)
        self.set_text_color(148, 163, 184) # Slate 400
        self.cell(0, 10, f"Page {self.page_no()}/{{nb}}", align="C")

def clean_text(text):
    """
    Cleans text to be fully compatible with standard Latin-1 PDF encoding.
    Converts rich emojis and special characters into beautiful ASCII/Latin-1 text equivalents.
    This prevents ElevenReader encoding glitches (like 'ðŸŽ§' instead of '🎧').
    """
    replacements = {
        '🎧': '[AUDIO CHRONICLE]',
        '💾': '[TECHNICAL STEMS]',
        '✅': '[OK]',
        '🔉': '[TTS]',
        '🔸': '*',
        '—': ' -- ',
        '–': ' -- ',
        '“': '"',
        '”': '"',
        '‘': "'",
        '’': "'",
        '…': '...',
        '•': '*',
        '│': '|',
        '─': '-',
        '┌': '+',
        '┐': '+',
        '└': '+',
        '┘': '+',
        '┬': '+',
        '┴': '+',
        '┼': '+',
        '├': '+',
        '┤': '+',
    }
    for char, replacement in replacements.items():
        text = text.replace(char, replacement)
    
    # Strip any other non-latin-1 characters to avoid FPDF crash
    cleaned = []
    for char in text:
        if ord(char) < 256:
            cleaned.append(char)
        else:
            cleaned.append('?')
    return "".join(cleaned)

def convert_md_to_pdf(md_path, pdf_path):
    if not os.path.exists(md_path):
        print(f"Error: Input markdown path does not exist: {md_path}")
        return False
        
    pdf = MarkdownPDF()
    pdf.alias_nb_pages()
    pdf.add_page()
    pdf.set_margins(20, 20, 20)
    pdf.set_auto_page_break(auto=True, margin=20)
    
    # Cover page style elements on first page
    pdf.set_y(25)
    
    with open(md_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
        
    in_code_block = False
    in_table = False
    headers = []
    
    for line in lines:
        line_str = line.strip()
        
        # Code blocks
        if line_str.startswith("```"):
            in_code_block = not in_code_block
            if in_code_block:
                pdf.ln(2)
            else:
                pdf.ln(2)
            continue
            
        if in_code_block:
            pdf.set_font("Courier", size=8.5)
            pdf.set_text_color(51, 65, 85) # Slate 700
            pdf.set_fill_color(248, 250, 252) # Slate 50 background
            pdf.set_x(25)
            pdf.multi_cell(160, 5, clean_text(line.rstrip('\n')), fill=True)
            continue
            
        # Table conversion (converts markdown table pipes into spoken text paragraphs for ElevenReader)
        if line_str.startswith("|") and line_str.endswith("|"):
            parts = [p.strip() for p in line_str.split("|")[1:-1]]
            if len(parts) >= 2:
                # Skip separator lines like |---|---|
                if all(all(c == '-' or c == ' ' for c in part) for part in parts):
                    continue
                
                if not in_table:
                    # Capture header row
                    in_table = True
                    headers = [p.strip() for p in parts]
                    pdf.ln(2)
                    pdf.set_font("Helvetica", "B", 10.5)
                    pdf.set_text_color(30, 41, 59)
                    pdf.multi_cell(0, 6, "Reference Details:")
                    pdf.ln(1)
                    continue
                else:
                    # Render row data as readable spoken bullet prose for ElevenReader
                    pdf.set_font("Helvetica", "", 9.5)
                    pdf.set_text_color(51, 65, 85)
                    
                    first_val = parts[0]
                    first_val = re.sub(r'\*\*(.*?)\*\*', r'\1', first_val)
                    first_val = re.sub(r'`(.*?)`', r'\1', first_val)
                    
                    # Print primary line
                    pdf.set_x(25)
                    pdf.set_font("Helvetica", "B", 9.5)
                    pdf.write(5.5, f"- {headers[0]}: ")
                    pdf.set_font("Helvetica", "", 9.5)
                    pdf.write(5.5, f"{clean_text(first_val)}\n")
                    
                    # Print sub-attributes
                    for i in range(1, len(parts)):
                        if i < len(headers):
                            sub_val = parts[i]
                            sub_val = re.sub(r'\*\*(.*?)\*\*', r'\1', sub_val)
                            sub_val = re.sub(r'`(.*?)`', r'\1', sub_val)
                            
                            pdf.set_x(30)
                            pdf.set_font("Helvetica", "B", 8.5)
                            pdf.write(4.5, f"  * {headers[i]}: ")
                            pdf.set_font("Helvetica", "", 8.5)
                            pdf.write(4.5, f"{clean_text(sub_val)}\n")
                    pdf.ln(2)
            continue
        else:
            if in_table:
                # Reset table state when exiting table block
                in_table = False
                headers = []
                pdf.ln(2)
                
        # Headers
        if line_str.startswith("# "):
            pdf.ln(8)
            pdf.set_font("Helvetica", "B", 20)
            pdf.set_text_color(15, 23, 42) # Slate 900
            pdf.multi_cell(0, 10, clean_text(line_str[2:]))
            pdf.ln(2)
            pdf.set_draw_color(99, 102, 241) # Indigo 500 accent
            pdf.set_line_width(0.8)
            pdf.line(pdf.get_x(), pdf.get_y(), pdf.get_x() + 45, pdf.get_y())
            pdf.set_line_width(0.2)
            pdf.ln(6)
            
        elif line_str.startswith("## "):
            pdf.ln(6)
            pdf.set_font("Helvetica", "B", 13.5)
            pdf.set_text_color(30, 41, 59) # Slate 800
            pdf.multi_cell(0, 8, clean_text(line_str[3:]))
            pdf.ln(2)
            
        elif line_str.startswith("### "):
            pdf.ln(4)
            pdf.set_font("Helvetica", "B", 11)
            pdf.set_text_color(71, 85, 105) # Slate 600
            pdf.multi_cell(0, 6, clean_text(line_str[4:]))
            pdf.ln(2)
            
        # Bullet list
        elif line_str.startswith("- ") or line_str.startswith("* "):
            pdf.set_font("Helvetica", "", 9.5)
            pdf.set_text_color(51, 65, 85) # Slate 700
            pdf.set_x(25)
            pdf.cell(5, 5.5, "-", ln=0)
            content = line_str[2:]
            
            content = re.sub(r'\*\*(.*?)\*\*', r'\1', content)
            content = re.sub(r'\*(.*?)\*', r'\1', content)
            content = re.sub(r'`(.*?)`', r'\1', content)
            
            pdf.multi_cell(0, 5.5, clean_text(content))
            
        # Horizontal rule
        elif line_str == "---":
            pdf.ln(4)
            pdf.set_draw_color(226, 232, 240) # Slate 200
            pdf.line(pdf.get_x(), pdf.get_y(), 190, pdf.get_y())
            pdf.ln(4)
            
        # Standard paragraph block
        elif line_str:
            is_meta = line_str.startswith("**Saved:**") or line_str.startswith("**Project:**") or line_str.startswith("**Status:**")
            if is_meta:
                pdf.set_font("Helvetica", "B", 9)
                pdf.set_text_color(100, 116, 139) # Slate 500
            else:
                pdf.set_font("Helvetica", "", 10)
                pdf.set_text_color(51, 65, 85) # Slate 700
                
            content = line_str
            content = re.sub(r'\*\*(.*?)\*\*', r'\1', content)
            content = re.sub(r'\*(.*?)\*', r'\1', content)
            content = re.sub(r'`(.*?)`', r'\1', content)
            
            pdf.multi_cell(0, 6, clean_text(content))
            pdf.ln(3)
        else:
            pdf.ln(2.5)
            
    pdf.output(pdf_path)
    print(f"Successfully generated PDF: {pdf_path}")
    return True

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python convert_to_pdf.py <input_md> <output_pdf>")
        sys.exit(1)
        
    convert_md_to_pdf(sys.argv[1], sys.argv[2])
