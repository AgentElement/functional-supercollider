#!/usr/bin/env python3
# test_alchemy.py

import sys
import traceback


def die(msg: str):
    print(msg)
    sys.exit(1)


def main():
    # ---------- Import ----------
    try:
        import alchemy
        import inspect
        print("✅ Imported alchemy")
        print("   module:", getattr(alchemy, "__file__", "<unknown>"))
        print("   PyFontanaGen.from_config signature:",
              inspect.signature(alchemy.PyFontanaGen.from_config))
    except Exception:
        die("❌ Failed to import 'alchemy'\n" + traceback.format_exc())

    # ---------- Utilities ----------
    try:
        raw = b"\x00\x01\xab\xcd"
        hx = alchemy.decode_hex_py("0001abcd")
        assert bytes(hx) == raw, f"decode_hex_py mismatch: {hx}"
        re_hx = alchemy.encode_hex_py(list(raw))
        assert re_hx.lower() == "0001abcd", f"encode_hex_py mismatch: {re_hx}"
        print("✅ Utilities OK (decode_hex_py / encode_hex_py)")
    except Exception:
        die("❌ Utilities failed\n" + traceback.format_exc())

    # ---------- Standardization ----------
    try:
        std_prefix = alchemy.PyStandardization("prefix")
        std_postfix = alchemy.PyStandardization("postfix")
        std_none    = alchemy.PyStandardization("none")
        for s in (std_prefix, std_postfix, std_none):
            assert s is not None
        print("✅ PyStandardization constructed (prefix/postfix/none)")
    except Exception:
        die("❌ PyStandardization construction failed\n" + traceback.format_exc())

    # ---------- Generators: BTreeGen ----------
    try:
        bt = alchemy.PyBTreeGen.from_config(
            size=6,
            freevar_generation_probability=0.3,
            max_free_vars=3,
            std=std_prefix,
        )
        one = bt.generate()
        many = bt.generate_n(5)
        assert isinstance(one, str) and len(one) > 0, "BTreeGen.generate must return a non-empty string"
        assert isinstance(many, list) and len(many) == 5 and all(isinstance(x, str) and x for x in many)
        print("✅ PyBTreeGen.generate / generate_n OK")
    except Exception:
        die("❌ PyBTreeGen tests failed\n" + traceback.format_exc())

    # ---------- Soup lifecycle ----------
    try:
        soup1 = alchemy.PySoup()
        assert soup1.len() == 0, "New soup should be empty"

        reactor = alchemy.PyReactor()
        soup2 = alchemy.PySoup.from_config(reactor)
        assert soup2.len() == 0, "Soup.from_config should start empty"

        exprs = bt.generate_n(10)
        soup2.perturb(exprs)
        assert soup2.len() == 10, f"After perturb, expected 10 expressions, got {soup2.len()}"

        steps = soup2.simulate_for(25, False)
        assert isinstance(steps, int)

        all_exprs = soup2.expressions()
        uniq = soup2.unique_expressions()
        counts = soup2.expression_counts()
        ent = soup2.population_entropy()
        col = soup2.collisions()
        ln = soup2.len()

        assert isinstance(all_exprs, list)
        assert isinstance(uniq, list)
        assert isinstance(counts, list) and all(
            isinstance(p, (tuple, list)) and len(p) == 2 and isinstance(p[0], str) and isinstance(p[1], int)
            for p in counts
        )
        assert isinstance(ent, float)
        assert isinstance(col, int)
        assert isinstance(ln, int)
        print(f"✅ PySoup lifecycle OK | len={ln}, uniq={len(uniq)}, collisions={col}, entropy={ent:.4f}")
    except Exception:
        die("❌ PySoup tests failed\n" + traceback.format_exc())

    # ---------- FontanaGen (new API + always returns str) ----------
    try:
        fg = alchemy.PyFontanaGen.from_config(
            abs_range=(0.2, 0.6),
            app_range=(0.2, 0.6),
            min_depth=1,              # test minimum nesting before allowing variables
            max_depth=5,              # cap depth growth
            free_variable_probability=0.25,
            max_free_vars=3,
        )
        term = fg.generate()
        assert isinstance(term, str) and term, "FontanaGen.generate must return a non-empty string"
        terms = fg.generate_n(5)
        assert isinstance(terms, list) and len(terms) == 5 and all(isinstance(t, str) and t for t in terms)
        print("✅ PyFontanaGen.from_config / generate / generate_n OK")
    except Exception:
        die("❌ PyFontanaGen tests failed\n" + traceback.format_exc())

    print("\n🎉 All python.rs bindings exercised successfully.")


if __name__ == "__main__":
    main()
