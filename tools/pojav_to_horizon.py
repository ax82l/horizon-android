#!/usr/bin/env python3
"""Convert a PojavLauncher control layout (controlmap/*.json) into a Horizon
control map ({"version": 1, "buttons": [...]}).

Horizon can also import a Pojav layout directly: Touch controls editor > Maps >
Import. This script is for converting on a computer, and generated the default map.

  python tools/pojav_to_horizon.py default.json -o out.json [--screen 2576x1159] [--density 2.75]
"""
import argparse, ast, json, operator, sys

LETTERS = "abcdefghijklmnopqrstuvwxyz"
ANDROID = {}
for i in range(10):
    ANDROID[7 + i] = f"key.keyboard.{i}"
for i, ch in enumerate(LETTERS):
    ANDROID[29 + i] = f"key.keyboard.{ch}"
for i in range(12):
    ANDROID[131 + i] = f"key.keyboard.f{i + 1}"
ANDROID.update({
    19: "key.keyboard.up", 20: "key.keyboard.down", 21: "key.keyboard.left",
    22: "key.keyboard.right", 55: "key.keyboard.comma", 56: "key.keyboard.period",
    57: "key.keyboard.left.alt", 58: "key.keyboard.right.alt",
    59: "key.keyboard.left.shift", 60: "key.keyboard.right.shift",
    61: "key.keyboard.tab", 62: "key.keyboard.space", 66: "key.keyboard.enter",
    67: "key.keyboard.backspace", 68: "key.keyboard.grave.accent",
    69: "key.keyboard.minus", 70: "key.keyboard.equal",
    71: "key.keyboard.left.bracket", 72: "key.keyboard.right.bracket",
    73: "key.keyboard.backslash", 74: "key.keyboard.semicolon",
    75: "key.keyboard.apostrophe", 76: "key.keyboard.slash",
    92: "key.keyboard.page.up", 93: "key.keyboard.page.down",
    111: "key.keyboard.escape", 112: "key.keyboard.delete",
    113: "key.keyboard.left.control", 114: "key.keyboard.right.control",
    115: "key.keyboard.caps.lock", 122: "key.keyboard.home", 123: "key.keyboard.end",
    124: "key.keyboard.insert", 143: "key.keyboard.num.lock",
    -1: "special:keyboard", -2: "special:hide_buttons", -3: "action:key.attack",
    -4: "action:key.use", -6: "action:key.pickItem",
})


def evaluate(expr, env, density):
    ops = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul,
           ast.Div: operator.truediv, ast.USub: operator.neg, ast.UAdd: operator.pos}
    for name, val in env.items():
        expr = expr.replace("${%s}" % name, "(%r)" % float(val))

    def ev(n):
        if isinstance(n, ast.Expression):
            return ev(n.body)
        if isinstance(n, ast.Constant) and isinstance(n.value, (int, float)):
            return float(n.value)
        if isinstance(n, ast.BinOp) and type(n.op) in ops:
            return ops[type(n.op)](ev(n.left), ev(n.right))
        if isinstance(n, ast.UnaryOp) and type(n.op) in ops:
            return ops[type(n.op)](ev(n.operand))
        if (isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
                and n.func.id == "px" and len(n.args) == 1):
            return ev(n.args[0]) * density
        raise ValueError("unsupported expression")
    return ev(ast.parse(expr, mode="eval"))


def convert(pojav, W, H, density):
    scale = float(pojav.get("scaledAt", 100.0))
    buttons, notes = [], []
    for b in pojav["mControlDataList"]:
        keys = []
        for code in b["keycodes"]:
            tok = ANDROID.get(code) if code else None
            if tok and tok not in keys:
                keys.append(tok)
        if not keys:
            notes.append((b.get("name", "?"), f"skipped: no equivalent for {b['keycodes']}"))
            continue
        wdp = b["width"] * scale / 100.0
        hdp = b["height"] * scale / 100.0
        w, h = wdp * density, hdp * density
        env = {"screen_width": W, "screen_height": H, "width": w, "height": h,
               "right": W - w, "bottom": H - h, "left": 0, "top": 0,
               "margin": 2.0 * density, "preferred_scale": scale}
        try:
            x = evaluate(b["dynamicX"], env, density)
            y = evaluate(b["dynamicY"], env, density)
        except Exception as e:
            notes.append((b.get("name", "?"), f"skipped: {e}"))
            continue
        buttons.append({
            "name": (b.get("name") or "")[:16],
            "keys": keys[:4],
            "center": [round((x + w / 2) / W, 4), round((y + h / 2) / H, 4)],
            "width": round(wdp, 4),
            "height": round(hdp, 4),
            "opacity": round(float(b.get("opacity", 1.0)), 3),
            "toggle": bool(b.get("isToggle")),
            "swipeable": bool(b.get("isSwipeable")),
            "pass_through": bool(b.get("passThruEnabled")),
            "show_in_game": bool(b.get("displayInGame", True)),
            "show_in_menu": bool(b.get("displayInMenu", True)),
            "background": int(b.get("bgColor", 0x4D000000)) & 0xFFFFFFFF,
            "stroke_color": int(b.get("strokeColor", -1)) & 0xFFFFFFFF,
            "stroke_width": float(b.get("strokeWidth", 0.0)),
            "corner_radius": float(b.get("cornerRadius", 0.0)),
        })
    return buttons, notes


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("pojav")
    ap.add_argument("-o", "--out", default="horizon-controlmap.json")
    ap.add_argument("--screen", default="2576x1159")
    ap.add_argument("--density", type=float, default=2.75)
    a = ap.parse_args()
    W, H = (float(v) for v in a.screen.lower().split("x"))
    with open(a.pojav, encoding="utf-8") as f:
        pojav = json.load(f)
    buttons, notes = convert(pojav, W, H, a.density)
    with open(a.out, "w", encoding="utf-8") as f:
        json.dump({"version": 1, "buttons": buttons[:128]}, f, indent=1, ensure_ascii=False)
    print(f"converted {len(buttons)} buttons -> {a.out}")
    for name, why in notes:
        print(f"  [{name}] {why}", file=sys.stderr)


if __name__ == "__main__":
    main()
