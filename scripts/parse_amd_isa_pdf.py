#!/usr/bin/env python3
"""
Extract instruction annotations from AMD ISA PDF reference guides.

Uses pdftotext (Poppler) to convert the PDF to text, then parses each
instruction entry into {description, operation, notes}.

Usage:
    python3 parse_amd_isa_pdf.py <pdf_path> -o <output.json>

Output JSON schema:
    {
      "s_add_i32": {
        "description": "Add two signed 32-bit integer inputs...",
        "operation": "tmp = S0.i + S1.i;\nSCC = ...;\nD0.i = tmp.i",
        "notes": "This opcode is not suitable for use with S_ADDC_U32..."
      },
      ...
    }
"""

import json
import re
import subprocess
import sys
from argparse import ArgumentParser


# ---------------------------------------------------------------------------
# MFMA name normalisation
# ---------------------------------------------------------------------------
# AMD ISA PDFs use names like V_MFMA_F32_16x16x1_4b_F32 where the optional
# _Nb multiplier suffix (4b, 2b, 16b) and the separating underscore before the
# data-type component are absent from LLVM TableGen mnemonic names
# (v_mfma_f32_16x16x1f32).  Apply this normalisation so PDF annotations map
# onto the correct tblgen-derived keys.
_MFMA_NORM_RE = re.compile(
    r'(_\d+[xX]\d+[xX]\d+)'    # dimension block e.g. _16x16x1
    r'(?:_\d+b)?'               # optional _Nb multiplier
    r'_([a-z])',                 # underscore + first char of dtype
)


def _normalize_mfma_key(name: str) -> str:
    """Normalise an MFMA/SMFMAC mnemonic from PDF naming to tblgen naming."""
    return _MFMA_NORM_RE.sub(lambda m: m.group(1) + m.group(2), name)


# ---------------------------------------------------------------------------
# Instruction name detection
# ---------------------------------------------------------------------------

INSTR_NAME_RE = re.compile(r'^([A-Z][A-Z0-9_]{3,})\s*\d*\s*$')

# Known ISA section/type header keywords that look like instruction names
# but are not instructions themselves.
_SECTION_HEADERS = {
    'SOP1', 'SOP2', 'SOPC', 'SOPK', 'SOPP',
    'VOP1', 'VOP2', 'VOP3', 'VOPC', 'VOP3P', 'VOPD',
    'DS', 'FLAT', 'MUBUF', 'MTBUF', 'MIMG', 'SMEM', 'SMRD',
    'EXP', 'VINTRP', 'VINTERP', 'LDSDIR', 'WMMA', 'MAI',
    'VALU', 'SALU', 'MFMA', 'CDNA', 'RDNA', 'GFX',
    'APPENDIX', 'GLOSSARY', 'INDEX', 'REVISION',
    # Common table column headers
    'ENCODING', 'FORMAT', 'OPCODE', 'MICROCODE', 'NOTES',
    'DESCRIPTION', 'OPERATION', 'OPERANDS', 'TYPE',
}

# All real AMDGPU instruction mnemonics start with one of these prefixes.
_PREFIXES = (
    'S_', 'V_', 'DS_', 'FLAT_', 'GLOBAL_', 'SCRATCH_',
    'BUFFER_', 'IMAGE_', 'TBUFFER_', 'VINTRP_',
    'EXP', 'S_NOP', 'S_ENDPGM', 'S_BRANCH',
)


def is_instruction_name(name: str) -> bool:
    """Return True if `name` looks like a real AMD GPU instruction mnemonic."""
    if name in _SECTION_HEADERS:
        return False
    return any(name.startswith(p) for p in _PREFIXES)


# ---------------------------------------------------------------------------
# Noise / footer detection
# ---------------------------------------------------------------------------

# Page footers in AMD ISA PDFs:
#   "16.1. SOP2 Instructions                           188 of 600"
#   '"RDNA3" Instruction Set Architecture'
#   '"AMD Instinct MI300" Instruction Set Architecture'
_FOOTER_SECTION_RE = re.compile(r'^\d+\.\d+')
_FOOTER_ISA_RE = re.compile(r'".*"\s+Instruction Set Architecture')
_PURE_NUMBER_RE = re.compile(r'^\d+\s*$')


def is_noise(line: str) -> bool:
    """Return True for page footer / header lines that should be discarded."""
    stripped = line.strip()
    if not stripped:
        return False
    if _PURE_NUMBER_RE.match(stripped):
        return True
    if _FOOTER_SECTION_RE.match(stripped):
        return True
    if _FOOTER_ISA_RE.search(stripped):
        return True
    return False


# ---------------------------------------------------------------------------
# Body parsing
# ---------------------------------------------------------------------------

_NOTES_RE = re.compile(r'^Notes?\s*$', re.IGNORECASE)
_CODE_HINTS = ('=', ';', 'if (', 'for (', 'while (', '//', 'else', 'return',
               'MEM[', 'CalcDs', 'RETURN_DATA', 'SCC', 'VCC', 'EXEC')


def _looks_like_code(line: str) -> bool:
    """Heuristic: does this (stripped) line look like pseudocode?"""
    return any(hint in line for hint in _CODE_HINTS)


def parse_body(lines: list[str]) -> dict:
    """
    Split collected body lines into description / operation / notes.

    Layout in AMD ISA PDFs:
      - Prose description paragraphs (left-aligned or minimally indented)
      - Blank lines
      - Pseudocode block (indented 2+ spaces)
      - Blank lines
      - "Notes" on its own line
      - Notes text
    """
    description_parts: list[str] = []
    operation_parts: list[str] = []
    notes_parts: list[str] = []

    state = 'description'  # description | operation | notes

    for raw in lines:
        if is_noise(raw):
            continue

        stripped = raw.strip()

        # "Notes" bare line → switch to notes state
        if _NOTES_RE.match(stripped):
            state = 'notes'
            continue

        # Indented line (2+ spaces) → operation / pseudocode
        if raw.startswith('  ') and stripped:
            if state in ('description', 'operation'):
                state = 'operation'
                operation_parts.append(stripped)
            elif state == 'notes':
                notes_parts.append(stripped)
            continue

        # Blank line
        if not stripped:
            if state == 'operation' and operation_parts:
                # A blank line after pseudocode ends the operation block,
                # but stay in operation state in case more code follows.
                operation_parts.append('')
            continue

        # Non-empty, non-indented line
        if state == 'description':
            description_parts.append(stripped)
        elif state == 'operation':
            # Non-indented after operation code: might be continuation
            # description (rare) or notes without the "Notes" header.
            if _looks_like_code(stripped):
                operation_parts.append(stripped)
            else:
                # Treat as notes
                state = 'notes'
                notes_parts.append(stripped)
        elif state == 'notes':
            notes_parts.append(stripped)

    def join(parts: list[str]) -> str:
        # Strip trailing blank lines, join.
        while parts and not parts[-1]:
            parts.pop()
        return '\n'.join(parts).strip()

    return {
        'description': join(description_parts),
        'operation':   join(operation_parts),
        'notes':       join(notes_parts),
    }


# ---------------------------------------------------------------------------
# PDF text extraction and instruction boundary detection
# ---------------------------------------------------------------------------

def extract_text(pdf_path: str) -> str:
    result = subprocess.run(
        ['pdftotext', '-layout', '-nopgbrk', pdf_path, '-'],
        capture_output=True, text=True, check=True,
    )
    return result.stdout


def parse_pdf(pdf_path: str) -> dict:
    """
    Parse one AMD ISA PDF and return a dict:
        { lowercase_mnemonic: {description, operation, notes} }
    """
    print(f'  Extracting text from {pdf_path} ...', file=sys.stderr)
    text = extract_text(pdf_path)
    lines = text.splitlines()

    annotations: dict[str, dict] = {}
    current_name: str | None = None
    body_lines: list[str] = []

    def _save(name: str, body: list[str]) -> None:
        ann = parse_body(body)
        if not (ann['description'] or ann['operation']):
            return
        raw_key = name.lower()
        # Apply MFMA dimension-suffix normalisation so PDF names
        # (v_mfma_f32_16x16x1_4b_f32) map to tblgen names (v_mfma_f32_16x16x1f32).
        key = _normalize_mfma_key(raw_key)
        # Store both the raw key (in case PDF and tblgen names already match) and
        # the normalised key, keeping the richer annotation on conflict.
        for k in ({key, raw_key} if key != raw_key else {key}):
            prev = annotations.get(k)
            if prev is None or (
                len(ann['description']) + len(ann['operation'])
                > len(prev['description']) + len(prev['operation'])
            ):
                annotations[k] = ann

    for line in lines:
        m = INSTR_NAME_RE.match(line)
        if m and is_instruction_name(m.group(1)):
            # Save the previous instruction's body
            if current_name and body_lines:
                _save(current_name, body_lines)
            current_name = m.group(1)
            body_lines = []
        elif current_name is not None:
            body_lines.append(line)

    # Flush the last instruction
    if current_name and body_lines:
        _save(current_name, body_lines)

    return annotations


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    ap = ArgumentParser(description='Parse AMD ISA PDF → instruction annotation JSON')
    ap.add_argument('pdf', help='Path to AMD ISA PDF')
    ap.add_argument('-o', '--output', required=True, help='Output JSON file path')
    args = ap.parse_args()

    annotations = parse_pdf(args.pdf)
    print(f'  Parsed {len(annotations)} instruction annotations.', file=sys.stderr)

    with open(args.output, 'w') as f:
        json.dump(annotations, f, indent=2, ensure_ascii=False)
    print(f'  Written to {args.output}', file=sys.stderr)


if __name__ == '__main__':
    main()
