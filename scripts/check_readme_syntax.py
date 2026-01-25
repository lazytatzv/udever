from pathlib import Path
p = Path('README.md')
text = p.read_text(encoding='utf-8')
lines = text.splitlines()

fence_count = 0
fence_stack = []
for line in lines:
    s = line.lstrip()
    if s.startswith('```'):
        fence = s.split()[0]
        if not fence_stack or fence_stack[-1] != fence:
            fence_stack.append(fence)
        else:
            fence_stack.pop()
        fence_count += 1

escaped = text.count('\\`')

outside_backticks = 0
in_fence = False
fence_curr = None
for line in lines:
    s = line.lstrip()
    if s.startswith('```'):
        if not in_fence:
            in_fence = True
            fence_curr = s.split()[0]
        else:
            in_fence = False
            fence_curr = None
        continue
    if not in_fence:
        outside_backticks += line.count('`')

errs = []
if fence_count % 2 != 0:
    errs.append(f"Unbalanced fenced code blocks (found {fence_count} fence markers)")
if escaped > 0:
    errs.append(f"Found {escaped} escaped backtick sequences (\\`) — consider removing backslashes")
if outside_backticks % 2 != 0:
    errs.append(f"Unbalanced inline backticks outside code fences (found {outside_backticks} backticks)")

if errs:
    print('SYNTAX-ISSUES')
    for e in errs:
        print('-', e)
    raise SystemExit(2)
print('OK: README.md syntax looks good (fenced blocks and inline backticks balanced)')
