#!/usr/bin/env python3
"""
Generate asm-lsp AMD GPU instruction JSON and register XML files from
an llvm-tblgen --dump-json dump of the AMDGPU target.

Usage:
    python3 gen_amdgpu_docs.py <path/to/amdgpu_all.json> <output_dir>

Where output_dir should contain docs_store/opcodes/ and docs_store/registers/
subdirectories (i.e. the repository root).
"""

import json
import os
import re
import sys
from collections import defaultdict


# ---------------------------------------------------------------------------
# Annotation helpers
# ---------------------------------------------------------------------------

def load_annotations(path: str) -> dict:
    """Load instruction annotations JSON; return {} if file absent."""
    try:
        with open(path) as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def build_summary(brief: str, annotation: dict | None) -> str:
    """Combine the auto-generated brief with PDF-extracted annotation.

    Returns markdown-formatted summary with description, operation pseudocode
    block, and notes sections when available; falls back to brief if not.
    """
    if not annotation:
        return brief
    desc = (annotation.get('description') or '').strip()
    op   = (annotation.get('operation')   or '').strip()
    notes = (annotation.get('notes')      or '').strip()
    # Require at least a meaningful description
    if len(desc) < 10:
        return brief
    parts = [desc]
    if op:
        parts.append('**Operation:**\n```\n' + op + '\n```')
    if notes:
        parts.append('**Notes:**\n' + notes)
    return '\n\n'.join(parts)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def clean_asm_string(asm: str) -> str:
    """Clean up tblgen AsmString into a readable template.

    Examples:
      'v_add_f32$vdst, $src0, $src1'  ->  'v_add_f32 vdst, src0, src1'
      'v_floor_f16$vdst, $src0_modifiers$clamp$omod ...' -> trimmed
    """
    if not asm:
        return ""
    # Insert a space before $operand when immediately after a mnemonic word
    # e.g. 'v_add_f32$vdst' -> 'v_add_f32 $vdst'
    asm = re.sub(r'([a-z0-9_])(\$)', r'\1 \2', asm)
    # Remove optional modifier suffixes like $clamp, $omod, $dpp_ctrl, etc.
    # after the main operands
    asm = re.sub(r'\s*\$(clamp|omod|dpp_ctrl|row_mask|bank_mask|bound_ctrl|fi|cpol|offset|gds|'
                 r'tfe|lwe|r128|a16|dim|unorm|dmask|glc|slc|dlc|nv|swz|scope|th|'
                 r'src0_sel|src1_sel|dst_sel|dst_unused|op_sel)[^,$\s]*', '', asm)
    # Remove {_e64} optional suffixes like 'v_cmpx_lg_f64{_e64}'
    asm = re.sub(r'\{[^}]+\}', '', asm)
    # Replace $varname with varname (strip the $)
    asm = re.sub(r'\$(\w+)', r'\1', asm)
    # Collapse multiple spaces
    asm = re.sub(r'  +', ' ', asm).strip()
    # Remove trailing commas or punctuation
    asm = asm.rstrip(',').strip()
    return asm


def mnemonic_from_asm(asm: str) -> str:
    """Extract the mnemonic (first word) from a cleaned AsmString."""
    if not asm:
        return ""
    # Handle cases like 'v_add_f32vdst, ...' (missing space after mnemonic)
    # Split on the first space or the first $ that was part of vdst
    parts = asm.split()
    if not parts:
        return ""
    # The mnemonic is up to the first non-alpha-underscore character if no space
    m = re.match(r'^([a-z_0-9]+)', parts[0], re.IGNORECASE)
    return m.group(1).lower() if m else parts[0].lower()


def describe_instruction(key: str, rec: dict, gen: str) -> str:
    """Build a short human-readable description for an instruction."""
    mnemonic = mnemonic_from_asm(rec.get('AsmString', '') or '')

    # Determine instruction class from superclasses
    supers = [s for s in (rec.get('!superclasses') or [])
              if isinstance(s, str)]
    # Common class name patterns
    class_hints = {
        'VOP1': 'Vector ALU (1 source)',
        'VOP2': 'Vector ALU (2 sources)',
        'VOP3': 'Vector ALU (3 operands, extended)',
        'VOPC': 'Vector ALU comparison',
        'VOP3P': 'Vector ALU packed/dot product',
        'SMRD': 'Scalar memory read',
        'MUBUF': 'Buffer memory (untyped)',
        'MTBUF': 'Buffer memory (typed)',
        'MIMG': 'Image memory instruction',
        'DS': 'LDS/GDS data share',
        'FLAT': 'Flat (global/scratch/LDS) memory',
        'SOP1': 'Scalar ALU (1 source)',
        'SOP2': 'Scalar ALU (2 sources)',
        'SOPC': 'Scalar ALU comparison',
        'SOPK': 'Scalar ALU with immediate',
        'SOPP': 'Scalar program flow',
        'EXP': 'Export instruction',
        'VINTRP': 'Vector interpolation',
        'VINTERP': 'Vector interpolation',
        'LDSDIR': 'LDS direct load',
        'VOPD': 'Dual-issue vector operation',
    }
    desc = ""
    for class_key, hint in class_hints.items():
        if class_key in supers:
            desc = hint
            break

    # Add MFMA / matrix hints
    if 'IsMAI' in supers or rec.get('IsMAI'):
        desc = "Matrix FMA (MFMA) instruction"
    if 'IsDOT' in supers or rec.get('IsDOT'):
        desc = "Dot product instruction"
    if 'IsWMMA' in supers or rec.get('IsWMMA'):
        desc = "WMMA matrix instruction"

    gen_label = {
        'gfx908':   'GFX908 (CDNA1/MI100)',
        'gfx90a':   'GFX90A (CDNA2/MI200/MI250)',
        'gfx942':   'GFX942 (CDNA3/MI300)',
        'gfx950':   'GFX950 (CDNA4/MI350)',
        'gfx10':    'GFX10 (RDNA1/Navi1x)',
        'gfx10-3':  'GFX10.3 (RDNA2/Navi2x)',
        'gfx11':    'GFX11 (RDNA3)',
        'gfx11-5':  'GFX11.5 (RDNA3.5/Strix)',
        'gfx12':    'GFX12 (RDNA4)',
        'gfx1250':  'GFX1250 (CDNA4)',
        'gfx1251':  'GFX1251 (CDNA4)',
    }.get(gen, gen)

    if desc:
        return f"{desc}. Available on {gen_label}."
    return f"Available on {gen_label}."


def build_asm_template(rec: dict) -> str:
    """Build the assembly template string from the AsmString."""
    raw = rec.get('AsmString', '') or ''
    cleaned = clean_asm_string(raw)
    return cleaned.upper() if cleaned else ""


# ---------------------------------------------------------------------------
# Instruction extraction
# ---------------------------------------------------------------------------

def is_real_instruction(rec: dict) -> bool:
    """Return True if this record is a real (non-pseudo, non-codegen-only) instruction."""
    if rec.get('isPseudo', 1):
        return False
    if rec.get('isCodeGenOnly', 1):
        return False
    asm = rec.get('AsmString', '') or ''
    if not asm:
        return False
    return True


def extract_by_suffix(d: dict, instr_keys: set, suffix: str,
                      annotations: dict | None = None,
                      fallback_annotations: dict | None = None) -> list:
    """Extract real instructions whose record name ends with _{suffix}."""
    if annotations is None:
        annotations = {}
    if fallback_annotations is None:
        fallback_annotations = {}
    results = {}
    for key in instr_keys:
        if not (key.endswith(f'_{suffix}') or f'_{suffix}_' in key):
            continue
        rec = d.get(key, {})
        if not is_real_instruction(rec):
            continue
        raw_asm = rec.get('AsmString', '') or ''
        mnemonic = mnemonic_from_asm(clean_asm_string(raw_asm))
        if not mnemonic:
            continue
        template = build_asm_template(rec)
        brief = describe_instruction(key, rec, suffix)
        ann = annotations.get(mnemonic) or fallback_annotations.get(mnemonic)
        summary = build_summary(brief, ann)
        # Deduplicate: keep the first seen template per mnemonic
        if mnemonic not in results:
            results[mnemonic] = {
                'name': mnemonic,
                'summary': summary,
                'asm_templates': [template] if template else [],
            }
        elif template and template not in results[mnemonic]['asm_templates']:
            results[mnemonic]['asm_templates'].append(template)
    return sorted(results.values(), key=lambda x: x['name'])


def extract_by_predicate(d: dict, instr_keys: set, predicate_name: str, gen: str,
                         annotations: dict | None = None,
                         fallback_annotations: dict | None = None) -> list:
    """Extract real instructions that match a specific SubtargetPredicate."""
    if annotations is None:
        annotations = {}
    if fallback_annotations is None:
        fallback_annotations = {}
    results = {}
    for key in instr_keys:
        rec = d.get(key, {})
        if not is_real_instruction(rec):
            continue
        pred = rec.get('SubtargetPredicate', {})
        pred_printable = pred.get('printable', '') if isinstance(pred, dict) else ''
        if predicate_name not in pred_printable:
            continue
        raw_asm = rec.get('AsmString', '') or ''
        mnemonic = mnemonic_from_asm(clean_asm_string(raw_asm))
        if not mnemonic:
            continue
        template = build_asm_template(rec)
        brief = describe_instruction(key, rec, gen)
        ann = annotations.get(mnemonic) or fallback_annotations.get(mnemonic)
        summary = build_summary(brief, ann)
        if mnemonic not in results:
            results[mnemonic] = {
                'name': mnemonic,
                'summary': summary,
                'asm_templates': [template] if template else [],
            }
        elif template and template not in results[mnemonic]['asm_templates']:
            results[mnemonic]['asm_templates'].append(template)
    return sorted(results.values(), key=lambda x: x['name'])


def merge_instrs(*lists) -> list:
    """Merge multiple instruction lists, deduplicating by name."""
    merged = {}
    for instrs in lists:
        for instr in instrs:
            name = instr['name']
            if name not in merged:
                merged[name] = instr.copy()
            else:
                # Append new templates
                for tmpl in instr.get('asm_templates', []):
                    if tmpl and tmpl not in merged[name]['asm_templates']:
                        merged[name]['asm_templates'].append(tmpl)
    return sorted(merged.values(), key=lambda x: x['name'])


# ---------------------------------------------------------------------------
# Register generation
# ---------------------------------------------------------------------------

def generate_register_xml() -> str:
    """Generate the amdgpu.xml register file in the format expected by populate_registers().

    The generic XML parser expects:
      <InstructionSet name="...">
        <Register name="..." description="..." type="General Purpose Register" width="32 bits"/>
      </InstructionSet>

    Covers registers common to all modern AMDGPU generations (GFX9+):
    - VGPR v0-v255 (vector general purpose, 32-bit lanes)
    - AGPR a0-a255 (accumulation VGPR, for CDNA matrix ops)
    - SGPR s0-s105 (scalar general purpose, 32-bit)
    - TTMP ttmp0-ttmp15 (trap temp, privileged)
    - Special: exec, exec_lo, exec_hi, vcc, vcc_lo, vcc_hi, m0, scc, etc.
    """
    # Use "amdgpu-gfx11" as the ISA name -- must match a valid Arch strum serialization
    # so that populate_registers() can deserialize it. All 4 AMD variants share this file;
    # the correct arch key is set when setup_registers() loads the file with the variant's self.
    lines = ['<?xml version="1.0" encoding="utf-8"?>', '<InstructionSet name="amdgpu-gfx11">']

    def reg(name, desc, reg_type, width):
        # Use explicit open/close tags (not self-closing) so the XML parser sees Event::End
        return (f'  <Register name="{name}" description="{desc}" '
                f'type="{reg_type}" width="{width}">\n  </Register>')

    # VGPR v0-v255
    for i in range(256):
        lines.append(reg(
            f'v{i}',
            f'Vector General Purpose Register {i} (32-bit per lane, 64 lanes on wave64)',
            'General Purpose Register', '32 bits'
        ))

    # AGPR a0-a255 (CDNA accumulation registers)
    for i in range(256):
        lines.append(reg(
            f'a{i}',
            f'Accumulation Vector GPR {i} (32-bit per lane, used for MFMA/matrix instructions on CDNA hardware)',
            'General Purpose Register', '32 bits'
        ))

    # SGPR s0-s105 (106 SGPRs on GFX9+)
    for i in range(106):
        lines.append(reg(
            f's{i}',
            f'Scalar General Purpose Register {i} (32-bit, uniform across wavefront)',
            'General Purpose Register', '32 bits'
        ))

    # TTMP (trap temp registers, privileged)
    for i in range(16):
        lines.append(reg(
            f'ttmp{i}',
            f'Trap Temporary Register {i} (32-bit, used during trap/exception handling)',
            'Special Purpose Register', '32 bits'
        ))

    # Special registers
    specials = [
        ('exec',    'Execution Mask (64-bit, controls which lanes are active in a wavefront)', '64 bits'),
        ('exec_lo', 'Execution Mask low 32 bits (alias for wave32 or explicit access)', '32 bits'),
        ('exec_hi', 'Execution Mask high 32 bits (wave64 upper half)', '32 bits'),
        ('vcc',     'Vector Condition Code (64-bit, result of VOPC comparisons on wave64)', '64 bits'),
        ('vcc_lo',  'Vector Condition Code low 32 bits (wave32 vcc, or lower half of wave64)', '32 bits'),
        ('vcc_hi',  'Vector Condition Code high 32 bits (wave64 upper half)', '32 bits'),
        ('m0',      'M0: multipurpose scalar register (LDS addressing, interpolation, indirect branch target)', '32 bits'),
        ('scc',     'Scalar Condition Code (1-bit, result of SOPC/SOP2 comparisons)', '32 bits'),
        ('flat_scratch',    'Flat Scratch base address register (64-bit base for private/scratch memory)', '64 bits'),
        ('flat_scratch_lo', 'Flat Scratch base address low 32 bits', '32 bits'),
        ('flat_scratch_hi', 'Flat Scratch base address high 32 bits', '32 bits'),
        ('xnack_mask',    'XNACK retry mask (64-bit, tracks pending memory retry on XNACK-enabled hardware)', '64 bits'),
        ('xnack_mask_lo', 'XNACK retry mask low 32 bits', '32 bits'),
        ('xnack_mask_hi', 'XNACK retry mask high 32 bits', '32 bits'),
        ('src_shared_base',  'LDS/shared memory base address', '32 bits'),
        ('src_shared_limit', 'LDS/shared memory limit address', '32 bits'),
        ('src_private_base', 'Private/scratch memory base address', '32 bits'),
        ('src_private_limit','Private/scratch memory limit address', '32 bits'),
        ('src_pops_exiting_wave_id', 'POPS (Primitive Ordered Pixel Shading) exiting wave ID', '32 bits'),
    ]
    for name, desc, width in specials:
        lines.append(reg(name, desc, 'Special Purpose Register', width))

    lines.append('</InstructionSet>')
    return '\n'.join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <amdgpu_all.json> <repo_root>", file=sys.stderr)
        sys.exit(1)

    json_path = sys.argv[1]
    repo_root = sys.argv[2]

    opcodes_dir = os.path.join(repo_root, 'docs_store', 'opcodes')
    regs_dir = os.path.join(repo_root, 'docs_store', 'registers')
    os.makedirs(opcodes_dir, exist_ok=True)
    os.makedirs(regs_dir, exist_ok=True)

    print(f"Loading {json_path} ...")
    with open(json_path) as f:
        d = json.load(f)

    instanceof = d.get('!instanceof', {})
    instr_keys = set(instanceof.get('Instruction', []))
    print(f"Total TableGen Instruction records: {len(instr_keys)}")

    # Load PDF-extracted annotations (gracefully absent)
    ann_dir = os.path.join(repo_root, 'docs_store', 'annotations')
    ann_gfx11   = load_annotations(os.path.join(ann_dir, 'amdgpu-gfx11-annotations.json'))
    ann_gfx950  = load_annotations(os.path.join(ann_dir, 'amdgpu-gfx950-annotations.json'))
    ann_gfx12   = load_annotations(os.path.join(ann_dir, 'amdgpu-gfx12-annotations.json'))
    ann_gfx1250 = load_annotations(os.path.join(ann_dir, 'amdgpu-gfx1250-annotations.json'))
    print(f"Loaded annotations: gfx11={len(ann_gfx11)}, gfx950={len(ann_gfx950)}, "
          f"gfx12={len(ann_gfx12)}, gfx1250={len(ann_gfx1250)}")

    # Build a merged pool: for any mnemonic appearing in multiple PDFs, keep the
    # entry with the richest description.  This lets GFX950 entries borrow from
    # the CDNA4 PDF (which documents shared MFMA variants in more detail) and
    # vice-versa, improving coverage without mixing generation-specific notes.
    def _merge_ann(*dicts):
        merged: dict = {}
        for d in dicts:
            for k, v in d.items():
                if k not in merged or (
                    len(v.get('description', '')) + len(v.get('operation', ''))
                    > len(merged[k].get('description', '')) + len(merged[k].get('operation', ''))
                ):
                    merged[k] = v
        return merged

    ann_all = _merge_ann(ann_gfx11, ann_gfx950, ann_gfx12, ann_gfx1250)
    print(f"Merged annotation pool: {len(ann_all)} unique mnemonics")

    # ----- GFX11 (RDNA3 / CDNA2-3) -----
    print("Extracting GFX11 instructions ...")
    gfx11_instrs = extract_by_suffix(d, instr_keys, 'gfx11',
                                     annotations=ann_gfx11,
                                     fallback_annotations=ann_all)
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx11.json')
    with open(out_path, 'w') as f:
        json.dump(gfx11_instrs, f, indent=2)
    print(f"  -> {len(gfx11_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX950 (CDNA4 / MI350) -----
    # GFX950 is the CDNA4 GPU for Instinct MI350 (ISA 9.5.0, GFX9-family encoding).
    # Base: GFX9/vi instructions + unique GFX950 predicate instructions.
    print("Extracting GFX950 instructions ...")
    gfx9_instrs = extract_by_suffix(d, instr_keys, 'gfx9',
                                    annotations=ann_gfx950,
                                    fallback_annotations=ann_all)
    vi_instrs   = extract_by_suffix(d, instr_keys, 'vi',
                                    annotations=ann_gfx950,
                                    fallback_annotations=ann_all)
    gfx950_specific = extract_by_predicate(d, instr_keys, 'HasGFX950Insts', 'gfx950',
                                           annotations=ann_gfx950,
                                           fallback_annotations=ann_all)
    gfx950_instrs = merge_instrs(gfx9_instrs, vi_instrs, gfx950_specific)
    # For instructions without annotations, add GFX950 availability note
    gfx950_names = {i['name'] for i in gfx950_specific}
    for instr in gfx950_instrs:
        summary = instr['summary']
        # Only rewrite if it's the terse auto-generated brief (starts with a class label
        # or "Available on") — annotated summaries start with prose description.
        is_brief = summary.startswith('Available on') or any(
            summary.startswith(h) for h in (
                'Scalar', 'Vector', 'Buffer', 'Image', 'LDS', 'Flat',
                'Export', 'Matrix', 'Dot', 'WMMA', 'Dual',
            )
        )
        if is_brief:
            if instr['name'] in gfx950_names:
                instr['summary'] = describe_instruction(instr['name'], {}, 'gfx950')
            elif 'gfx950' not in summary:
                instr['summary'] = summary.replace(
                    'Available on', 'Available on GFX950 (CDNA4/MI350); originally'
                )
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx950.json')
    with open(out_path, 'w') as f:
        json.dump(gfx950_instrs, f, indent=2)
    print(f"  -> {len(gfx950_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX12 (RDNA4) -----
    print("Extracting GFX12 instructions ...")
    gfx12_instrs = extract_by_suffix(d, instr_keys, 'gfx12',
                                     annotations=ann_gfx12,
                                     fallback_annotations=ann_all)
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx12.json')
    with open(out_path, 'w') as f:
        json.dump(gfx12_instrs, f, indent=2)
    print(f"  -> {len(gfx12_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX1250 (CDNA4) -----
    # GFX1250 is a GFX12-family chip with additional CDNA4-specific instructions.
    # Include all GFX12 base instructions plus GFX1250-specific ones.
    print("Extracting GFX1250 instructions ...")
    gfx1250_specific = extract_by_suffix(d, instr_keys, 'gfx1250',
                                         annotations=ann_gfx1250,
                                         fallback_annotations=ann_all)
    gfx1250_pred_instrs = extract_by_predicate(d, instr_keys, 'GFX125x', 'gfx1250',
                                               annotations=ann_gfx1250,
                                               fallback_annotations=ann_all)
    gfx1250_instrs = merge_instrs(gfx12_instrs, gfx1250_specific, gfx1250_pred_instrs)
    # Update summaries for GFX1250-specific ones without annotations
    gfx1250_names = {i['name'] for i in gfx1250_specific} | {i['name'] for i in gfx1250_pred_instrs}
    for instr in gfx1250_instrs:
        summary = instr['summary']
        is_brief = summary.startswith('Available on') or any(
            summary.startswith(h) for h in (
                'Scalar', 'Vector', 'Buffer', 'Image', 'LDS', 'Flat',
                'Export', 'Matrix', 'Dot', 'WMMA', 'Dual',
            )
        )
        if is_brief and instr['name'] in gfx1250_names:
            instr['summary'] = describe_instruction(instr['name'], {}, 'gfx1250')
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx1250.json')
    with open(out_path, 'w') as f:
        json.dump(gfx1250_instrs, f, indent=2)
    print(f"  -> {len(gfx1250_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX908 (CDNA1 / MI100) -----
    # gfx908 = GFX9 base ISA + MAI/MFMA instructions (gfx908 was first to add MFMA).
    # tblgen does not gate gfx908-specific records by a dedicated suffix or
    # predicate; the CDNA1 ISA is the union of GFX9-suffixed records plus the
    # MAI ones whose predicate string mentions HasMAIInsts (without requiring
    # GFX90APlus/GFX940Plus, which would mean a newer chip).
    print("Extracting GFX908 instructions ...")
    vi_for_908 = extract_by_suffix(d, instr_keys, 'vi',
                                   annotations={}, fallback_annotations=ann_all)
    gfx9_for_908 = extract_by_suffix(d, instr_keys, 'gfx9',
                                     annotations={}, fallback_annotations=ann_all)
    mai_instrs = extract_by_predicate(d, instr_keys, 'HasMAIInsts', 'gfx908',
                                      annotations={}, fallback_annotations=ann_all)
    gfx908_instrs = merge_instrs(vi_for_908, gfx9_for_908, mai_instrs)
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx908.json')
    with open(out_path, 'w') as f:
        json.dump(gfx908_instrs, f, indent=2)
    print(f"  -> {len(gfx908_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX90A (CDNA2 / MI200/MI250) -----
    # gfx90a inherits gfx908 and adds packed FP ops + extra MFMA variants.
    print("Extracting GFX90A instructions ...")
    gfx90a_suffix = extract_by_suffix(d, instr_keys, 'gfx90a',
                                      annotations={}, fallback_annotations=ann_all)
    gfx90a_only_pred = extract_by_predicate(d, instr_keys, 'isGFX90AOnly', 'gfx90a',
                                            annotations={}, fallback_annotations=ann_all)
    gfx90a_instrs = merge_instrs(gfx908_instrs, gfx90a_suffix, gfx90a_only_pred)
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx90a.json')
    with open(out_path, 'w') as f:
        json.dump(gfx90a_instrs, f, indent=2)
    print(f"  -> {len(gfx90a_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX942 (CDNA3 / MI300) -----
    # gfx942 inherits gfx90a and adds gfx940-suffixed records and isGFX940Plus
    # ones. We exclude GFX950-only records to keep gfx942 distinct from gfx950.
    print("Extracting GFX942 instructions ...")
    gfx940_suffix = extract_by_suffix(d, instr_keys, 'gfx940',
                                      annotations={}, fallback_annotations=ann_all)
    gfx940plus_pred = extract_by_predicate(d, instr_keys, 'isGFX940Plus', 'gfx942',
                                           annotations={}, fallback_annotations=ann_all)
    # Filter out GFX950-only items from the predicate sweep
    gfx950_only_names = {i['name'] for i in extract_by_predicate(
        d, instr_keys, 'HasGFX950Insts', 'gfx950',
        annotations={}, fallback_annotations=ann_all)}
    gfx940plus_pred = [i for i in gfx940plus_pred if i['name'] not in gfx950_only_names]
    gfx942_instrs = merge_instrs(gfx90a_instrs, gfx940_suffix, gfx940plus_pred)
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx942.json')
    with open(out_path, 'w') as f:
        json.dump(gfx942_instrs, f, indent=2)
    print(f"  -> {len(gfx942_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX10 (RDNA1 / Navi1x: gfx1010-1013) -----
    # tblgen does not differentiate RDNA1 vs RDNA2 base ISA at the record
    # level (everything sits under isGFX10Only / suffix _gfx10), so RDNA1 and
    # RDNA2 share the same extraction. We still emit two separate JSON files
    # so users can pick the family-correct ISA name in their config.
    print("Extracting GFX10 instructions ...")
    gfx10_instrs = extract_by_suffix(d, instr_keys, 'gfx10',
                                     annotations={}, fallback_annotations=ann_all)
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx10.json')
    with open(out_path, 'w') as f:
        json.dump(gfx10_instrs, f, indent=2)
    print(f"  -> {len(gfx10_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX10.3 (RDNA2 / Navi2x: gfx1030-1036) -----
    print("Extracting GFX10.3 instructions ...")
    gfx10_3_instrs = list(gfx10_instrs)
    # Re-tag the gen label in any auto-generated brief summaries
    for instr in gfx10_3_instrs:
        if 'GFX10 (RDNA1' in instr['summary']:
            instr['summary'] = instr['summary'].replace(
                'GFX10 (RDNA1/Navi1x)', 'GFX10.3 (RDNA2/Navi2x)'
            )
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx10-3.json')
    with open(out_path, 'w') as f:
        json.dump(gfx10_3_instrs, f, indent=2)
    print(f"  -> {len(gfx10_3_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX11.5 (RDNA3.5 / Strix: gfx1150-1153) -----
    # No dedicated tblgen predicate; share the GFX11 instruction set.
    print("Extracting GFX11.5 instructions ...")
    gfx11_5_instrs = list(gfx11_instrs)
    for instr in gfx11_5_instrs:
        if 'GFX11 (RDNA3' in instr['summary']:
            instr['summary'] = instr['summary'].replace(
                'GFX11 (RDNA3)', 'GFX11.5 (RDNA3.5/Strix)'
            )
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx11-5.json')
    with open(out_path, 'w') as f:
        json.dump(gfx11_5_instrs, f, indent=2)
    print(f"  -> {len(gfx11_5_instrs)} unique mnemonics written to {out_path}")

    # ----- GFX1251 (CDNA4 sibling of gfx1250) -----
    # tblgen treats gfx1250 and gfx1251 as the same instruction set
    # (isGFX125xOnly / FeatureGFX1250Insts). Emit a parallel JSON so the
    # Arch::AmdgpuGfx1251 variant can carry its own bincode.
    print("Extracting GFX1251 instructions ...")
    gfx1251_instrs = list(gfx1250_instrs)
    for instr in gfx1251_instrs:
        if 'GFX1250' in instr['summary']:
            instr['summary'] = instr['summary'].replace('GFX1250', 'GFX1251')
    out_path = os.path.join(opcodes_dir, 'amdgpu-gfx1251.json')
    with open(out_path, 'w') as f:
        json.dump(gfx1251_instrs, f, indent=2)
    print(f"  -> {len(gfx1251_instrs)} unique mnemonics written to {out_path}")

    # ----- Shared register XML -----
    print("Generating register definitions ...")
    reg_xml = generate_register_xml()
    out_path = os.path.join(regs_dir, 'amdgpu.xml')
    with open(out_path, 'w') as f:
        f.write(reg_xml)
    print(f"  -> Register XML written to {out_path}")

    print("Done.")


if __name__ == '__main__':
    main()
