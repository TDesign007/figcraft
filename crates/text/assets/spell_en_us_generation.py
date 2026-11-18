#!/usr/bin/env python3
"""Expand the hunspell en_US dictionary (SCOWL) into a plain lowercase word list.

Reads en_US.aff and en_US.dic from /tmp/scowl/en_us and writes one word per line to
/tmp/scowl/spell_en_us.txt. Simplified but adequate for spell checking: applies every
PFX/SFX rule whose condition matches the stem, with cross product; follows continuation
flags attached to an affix; skips ONLYINCOMPOUND entries and entries with digits.
"""
import re
import sys

AFF = "/tmp/scowl/en_us/en_US.aff"
DIC = "/tmp/scowl/en_us/en_US.dic"
OUT = "/tmp/scowl/spell_en_us.txt"


def cond_to_regex(cond: str, end: bool) -> re.Pattern:
    """Translate a hunspell affix condition to a regex.

    A suffix condition matches the rightmost characters of the stem, a prefix condition
    the leftmost ones (end selects which anchor to use).
    """
    if cond == ".":
        return re.compile(r"\A.*\Z", re.S)
    out = []
    i = 0
    while i < len(cond):
        c = cond[i]
        if c == "[":
            j = cond.index("]", i)
            cls = cond[i:j + 1]
            i = j + 1
        elif c == ".":
            cls = "."
            i += 1
        else:
            cls = re.escape(c)
            i += 1
        if i < len(cond) and cond[i] == "*":
            out.append(f"(?:{cls})*")
            i += 1
        else:
            out.append(cls)
    # A suffix condition anchors at the stem's end, a prefix condition at its start.
    pattern = "".join(out)
    return re.compile(pattern + r"\Z" if end else r"\A" + pattern, re.S)


def parse_aff():
    prefixes, suffixes = {}, {}
    onlyincompound = None
    lines = open(AFF, encoding="utf-8").read().splitlines()
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if line.startswith("ONLYINCOMPOUND"):
            onlyincompound = line.split()[1]
        parts = line.split()
        if parts and parts[0] in ("PFX", "SFX") and len(parts) >= 4 and parts[2] in ("Y", "N"):
            kind, flag, cross = parts[0], parts[1], parts[2] == "Y"
            count = int(parts[3])
            rules = []
            for r in lines[i + 1:i + 1 + count]:
                f = r.split()
                if len(f) < 4:
                    continue
                _, rflag, strip, affix = f[0], f[1], f[2], f[3]
                rest = f[4:]
                # The condition field follows the affix; a lone "0" there is a morph field
                # (a real condition is "." or a pattern), so treat it as ".".
                cond = "."
                if rest:
                    cond = rest[0] if (len(rest) > 1 or rest[0] != "0") else "."
                # Continuation flags ride on the affix after a slash.
                cont = ""
                if "/" in affix:
                    affix, cont = affix.split("/", 1)
                if affix == "0":
                    affix = ""
                if strip == "0":
                    strip = ""
                rules.append({
                    "strip": strip,
                    "affix": affix,
                    "re": cond_to_regex(cond, kind == "SFX"),
                    "cont": cont,
                })
            (prefixes if kind == "PFX" else suffixes)[flag] = {"cross": cross, "rules": rules}
            i += count
        i += 1
    return prefixes, suffixes, onlyincompound


def main():
    prefixes, suffixes, onlyincompound = parse_aff()
    words = set()

    def add(w):
        if w and not any(ch.isdigit() for ch in w):
            words.add(w.lower())

    def apply(flags, stem):
        """All forms of stem reachable through flags (with continuation recursion)."""
        forms = {stem}
        queue = [(flags, stem, True)]
        seen = set()
        while queue:
            fl, base, top = queue.pop(0)
            if (fl, base) in seen or len(seen) > 200:
                continue
            seen.add((fl, base))
            produced = []
            for flag in fl:
                table = suffixes.get(flag)
                if table:
                    for r in table["rules"]:
                        # A suffix condition describes the stem's last characters: search it.
                        if r["re"].search(base) and (base.endswith(r["strip"]) if r["strip"] else True):
                            form = base[: len(base) - len(r["strip"])] + r["affix"] if r["strip"] else base + r["affix"]
                            produced.append((form, r["cont"]))
                table = prefixes.get(flag)
                if table:
                    for r in table["rules"]:
                        if r["re"].match(base) and (base.startswith(r["strip"]) if r["strip"] else True):
                            form = r["affix"] + base[len(r["strip"]):]
                            produced.append((form, r["cont"]))
            for form, cont in produced:
                if form:
                    forms.add(form)
                if cont:
                    queue.append((cont, form, False))
        return forms

    data = open(DIC, encoding="utf-8").read().splitlines()
    total = 0
    for line in data[1:]:
        entry = line.split("/", 1)
        word = entry[0].strip()
        flags = entry[1].split()[0] if len(entry) > 1 else ""
        if not word or "/" in word:
            continue
        # Morph descriptions come after a tab; ignore them.
        word = word.split("\t")[0]
        if onlyincompound and onlyincompound in flags:
            continue
        total += 1
        for form in apply(flags, word):
            add(form)

    with open(OUT, "w", encoding="utf-8") as f:
        for w in sorted(words):
            f.write(w + "\n")
    print(f"entries: {total}, unique forms: {len(words)}")
    checks = ["running", "happily", "careful", "boxes", "walked", "bigger", "flying", "designer", "prototype", "don't"]
    present = {c: c in words for c in checks}
    print(present)


if __name__ == "__main__":
    main()
