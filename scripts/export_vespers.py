import os
import re
import glob
import shutil

# Paths
SRC_DIR = u'C:\\Users\\mkibb\\Documents\\deep-meteor\\architext-core\\vespers_extracted\\Private & Shared\\Area 52 \u2014 Vespers'
DEST_DIR = u'C:\\Users\\mkibb\\Documents\\deep-meteor\\architext-core\\vespers'

def clean_markdown_remnants(content):
    # Strip table of contents or other common Notion-exported meta blocks if needed
    # For now, return content cleaned of trailing/leading whitespace
    return content.strip()

def main():
    print("Starting Vespers local migration...")
    
    # Create directories
    for sub in ['plan', 'prose', 'reviews']:
        os.makedirs(os.path.join(DEST_DIR, sub), exist_ok=True)
        
    # 1. Create project.yaml
    project_yaml_content = """title: "Vespers"
author: "Paradroid"
revision_mode: "auto"
agent: "jonathan"
target_chapters: 13
point_of_view: "Third Person Limited"
pov_tense: "Past Tense"
prose_complexity: "Complex / Literary"
narrative_tone: "Gritty, clinical, cybernetic, worn-down"
core_themes: "Memory decay, systemic friction, wall breaches, survival under synthetic conditions"
setting_era_location: "Outpost Colony / Area 52"
setting_atmosphere: "Atmospheric, clinical, cold"
phases: []
"""
    with open(os.path.join(DEST_DIR, 'project.yaml'), 'w', encoding='utf-8') as f:
        f.write(project_yaml_content)
    print("Created project.yaml")

    # 2. Copy/Create concept files
    # philosophical alignment
    alignment_file = glob.glob(os.path.join(SRC_DIR, '*Philosophical Alignment Dock*.md'))
    if alignment_file:
        shutil.copy2(alignment_file[0], os.path.join(DEST_DIR, 'plan', '01_concept.md'))
        print("Copied Philosophical Alignment Dock -> plan/01_concept.md")
    
    # universe framework
    universe_file = glob.glob(os.path.join(SRC_DIR, '*Universe & Expansion*.md'))
    if universe_file:
        shutil.copy2(universe_file[0], os.path.join(DEST_DIR, 'plan', '02_universe.md'))
        print("Copied Universe & Expansion Framework -> plan/02_universe.md")
        
    # project reference
    reference_file = glob.glob(os.path.join(SRC_DIR, '*Project Reference*.md'))
    if reference_file:
        shutil.copy2(reference_file[0], os.path.join(DEST_DIR, 'plan', '03_plot.md'))
        print("Copied Project Reference -> plan/03_plot.md")

    # 3. Concatenate Act Blueprints -> plan/04_blueprint.md
    act1_blueprint = glob.glob(os.path.join(SRC_DIR, '*Act 1 \u2014 Creative Blueprint*.md'))
    act2_blueprint = glob.glob(os.path.join(SRC_DIR, '*Act 2 \u2014 Creative Blueprint*.md'))
    act3_4_blueprint = glob.glob(os.path.join(SRC_DIR, '*Act 3 & 4 \u2014 Blueprint*.md'))
    
    blueprint_content = []
    if act1_blueprint:
        with open(act1_blueprint[0], 'r', encoding='utf-8') as f:
            blueprint_content.append("# Act 1 Creative Blueprint\n\n" + f.read())
    if act2_blueprint:
        with open(act2_blueprint[0], 'r', encoding='utf-8') as f:
            blueprint_content.append("# Act 2 Creative Blueprint\n\n" + f.read())
    if act3_4_blueprint:
        with open(act3_4_blueprint[0], 'r', encoding='utf-8') as f:
            blueprint_content.append("# Act 3 & 4 Blueprint & Endpoint Definition\n\n" + f.read())
            
    if blueprint_content:
        with open(os.path.join(DEST_DIR, 'plan', '04_blueprint.md'), 'w', encoding='utf-8') as f:
            f.write("\n\n---\n\n".join(blueprint_content))
        print("Created plan/04_blueprint.md")

    # 4. Concatenate Reviews -> reviews/00_act_reviews.md
    review_files = [
        ('*Act 1 Review \u2014 Jonathan*.md', "Act 1 Review - Jonathan"),
        ('*Act 1 \u2014 Chapter Reviews & Peer Review*.md', "Act 1 - Chapter Reviews & Peer Review"),
        ('*Act 1 \u2014 Independent Peer Review*.md', "Act 1 - Independent Peer Review"),
        ('*Act 2 \u2014 Chapter Reviews*.md', "Act 2 - Chapter Reviews"),
        ('*Midpoint Review*.md', "Midpoint Review")
    ]
    
    reviews_content = []
    for pattern, title in review_files:
        matched = glob.glob(os.path.join(SRC_DIR, pattern))
        if matched:
            with open(matched[0], 'r', encoding='utf-8') as f:
                reviews_content.append(f"# {title}\n\n" + f.read())
                
    if reviews_content:
        with open(os.path.join(DEST_DIR, 'reviews', '00_act_reviews.md'), 'w', encoding='utf-8') as f:
            f.write("\n\n---\n\n".join(reviews_content))
        print("Created reviews/00_act_reviews.md")

    # 5. Process chapters 1 to 13
    for ch in range(1, 14):
        # Find chapter file (excluding special blueprints or reviews)
        ch_pattern = f"Chapter {ch} \u2014*.md"
        ch_files = glob.glob(os.path.join(SRC_DIR, ch_pattern))
        
        # Filter out Chapter 10 since it has two files
        if ch == 10:
            blueprint_file = glob.glob(os.path.join(SRC_DIR, "Chapter 10 \u2014 The Face d437063a*.md"))[0]
            prose_file = glob.glob(os.path.join(SRC_DIR, "Chapter 10 \u2014 The Face ff1abe49*.md"))[0]
            
            # Map d437063a entire content to plan/ch10_plan.md
            with open(blueprint_file, 'r', encoding='utf-8') as f:
                bp_content = f.read()
            with open(os.path.join(DEST_DIR, 'plan', 'ch10_plan.md'), 'w', encoding='utf-8') as f:
                f.write(bp_content)
            print("Extracted Chapter 10 Blueprint -> plan/ch10_plan.md")
            
            # Extract prose from ff1abe49
            with open(prose_file, 'r', encoding='utf-8') as f:
                full_content = f.read()
            
            # Find the header # Chapter 10: The Face
            prose_start_match = re.search(r'^#\s+Chapter\s+10:\s+The\s+Face', full_content, re.IGNORECASE | re.MULTILINE)
            if prose_start_match:
                prose_text = full_content[prose_start_match.start():]
                with open(os.path.join(DEST_DIR, 'prose', 'ch10_prose.md'), 'w', encoding='utf-8') as f:
                    f.write(prose_text)
                print("Extracted Chapter 10 Prose -> prose/ch10_prose.md")
            else:
                print("ERROR: Could not find prose start in Chapter 10 ff1abe49!")
            continue

        if not ch_files:
            print(f"Warning: Chapter {ch} file not found!")
            continue
            
        file_path = ch_files[0]
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            
        # Chapters 1-8: Prose only
        if ch in range(1, 9):
            # The entire file is prose
            with open(os.path.join(DEST_DIR, 'prose', f'ch{ch:02d}_prose.md'), 'w', encoding='utf-8') as f:
                f.write(content)
            # Create a simple stub plan
            stub_plan = f"# Chapter {ch} Plan\n\n*Initial plan migrated from original workspace.*"
            with open(os.path.join(DEST_DIR, 'plan', f'ch{ch:02d}_plan.md'), 'w', encoding='utf-8') as f:
                f.write(stub_plan)
            print(f"Processed Chapter {ch} (Prose-only)")
            
        # Chapters 9, 11-13: Contain both blueprint and prose
        elif ch in [9, 11, 12, 13]:
            # Look for blueprint header and prose header
            # Blueprint starts at ## Phase 4-5: Full Blueprint (or similar)
            # Prose starts at # Chapter X: <Name>
            bp_start_match = re.search(r'^##\s+Phase\s+4.*Blueprint', content, re.IGNORECASE | re.MULTILINE)
            prose_start_pattern = f"^#\\s+Chapter\\s+{ch}:"
            prose_start_match = re.search(prose_start_pattern, content, re.IGNORECASE | re.MULTILINE)
            
            if bp_start_match and prose_start_match:
                bp_text = content[bp_start_match.start():prose_start_match.start()]
                prose_text = content[prose_start_match.start():]
                
                # Write to plan and prose
                with open(os.path.join(DEST_DIR, 'plan', f'ch{ch:02d}_plan.md'), 'w', encoding='utf-8') as f:
                    f.write(bp_text)
                with open(os.path.join(DEST_DIR, 'prose', f'ch{ch:02d}_prose.md'), 'w', encoding='utf-8') as f:
                    f.write(prose_text)
                print(f"Processed Chapter {ch} (Split Blueprint & Prose)")
            else:
                # Fallback: write entire file to prose, create stub plan
                with open(os.path.join(DEST_DIR, 'prose', f'ch{ch:02d}_prose.md'), 'w', encoding='utf-8') as f:
                    f.write(content)
                stub_plan = f"# Chapter {ch} Plan\n\n*Initial plan migrated from original workspace.*"
                with open(os.path.join(DEST_DIR, 'plan', f'ch{ch:02d}_plan.md'), 'w', encoding='utf-8') as f:
                    f.write(stub_plan)
                print(f"Processed Chapter {ch} (Fallback - No split markers found)")

    # 6. Stub empty reviews for Chapters 1-13 as needed by daemon checks
    for ch in range(1, 14):
        review_path = os.path.join(DEST_DIR, 'reviews', f'ch{ch:02d}_review.md')
        if not os.path.exists(review_path):
            with open(review_path, 'w', encoding='utf-8') as f:
                f.write(f"# Chapter {ch} Review\n\n*Review placeholder.*")
                
    print("Migration completed successfully!")

if __name__ == '__main__':
    main()
