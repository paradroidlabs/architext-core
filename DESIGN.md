---
version: "alpha"
name: Architext Narrative IDE
description: The visual DNA for the Architext Core native eframe client
colors:
  primary: "#0B0C10"
  secondary: "#1F2833"
  tertiary: "#66FCF1"
  neutral: "#C5C6C7"
typography:
  h1:
    fontFamily: Fira Code
    fontSize: 2.5rem
  body-md:
    fontFamily: Inter
    fontSize: 1rem
  label-caps:
    fontFamily: Fira Code
    fontSize: 0.85rem
rounded:
  sm: 2px
  md: 4px
spacing:
  sm: 8px
  md: 16px
---

## Overview

Architext serves as the control room for narrative generation. The design system reflects a synthesis of low-level cybernetic terminals and classic editorial software. It is a heads-up display for the filesystem state machine. The UI evokes a precision-engineered workspace — dark, deeply contrasted, with sharp, luminous accents that guide orchestration without overwhelming the prose.

## Colors

The color palette establishes a nocturnal, high-focus environment. It relies on deep voids, structural grays, and a singular luminous accent color representing active agent operations.

The primary void creates the overarching spatial depth. The secondary charcoal provides structural boundaries for panels, logs, and structural separators. The tertiary luminous cyan serves as the primary vector of interaction, used exclusively for active agent states, critical buttons, and active phase indicators. The neutral light gray ensures long-form readability across all prose panels.

## Typography

Typography bridges the gap between code and literature. Monospace fonts are used for system structure, logs, and metadata, while sans-serif fonts carry the prose payload.

The primary interface components rely on monospaced geometry to reinforce the terminal aesthetic and ensure metadata aligns perfectly. Prose rendering drops the monospace rigidity in favor of highly legible, modern sans-serif typography that prevents fatigue during extended review sessions.

## Layout

The spatial organization is utilitarian and dense, prioritizing information density over negative space. 

Padding and margins are kept relatively tight to ensure the maximum amount of narrative context and filesystem status fits within the viewport. The interface heavily utilizes grid-based subdivision, where panels snap to rigid structural lines.

## Elevation & Depth

Architext minimizes vertical z-index layering, favoring a flat, single-plane layout. Depth is primarily communicated through border contrast and subtle shifts in the dark background colors rather than relying on drop shadows or complex lighting simulations. Panels are delineated by 1-pixel borders rather than floating above the canvas.

## Shapes

Corners are intentionally sharp to maintain the terminal aesthetic. Radius values are kept to an absolute minimum, ensuring that interface elements feel precise, mechanical, and rigid, aligning with the concept of the filesystem as a state machine.
