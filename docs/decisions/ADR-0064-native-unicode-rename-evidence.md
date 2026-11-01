# ADR-0064: Native Unicode rename and collision evidence

Date: 2026-11-01. Status: implemented; full local and Windows native checks pass. Target CI follows the checkpoint.

Extend the closed generated rename experiment with literal composed/decomposed accented media and companion names. Observe exact-parent name equivalence through exclusive native probes; never normalize or lowercase execution paths. Run interrupted staged rename, fresh continuation and reverse undo, then verify original objects, complete bytes, timestamps and directory inventory.

A separate generated pair targets canonically equivalent spellings. If the actual filesystem aliases those names, require a NativeNameCollision containing both participants, zero steps, execution refusal and zero journal events. If it distinguishes them, require distinct publication and reverse restoration. This is an observed branch rather than an OS-name assumption. Both branches verify all original members and retained probe objects.

The constructor helper remains private and all callers pass literal generated names. Existing normal fixture construction retains its conflict rejection; the collision experiment alone inspects a blocked plan, whose executor still refuses unresolved issues before a move intent. Existing five-member rename/job/UI contracts remain unchanged. Add the bounded Unicode receipt to the existing instrumented native fixture probe and assert it in both target desktop suites.

This closes part of AC-028/029 native evidence; it grants no production original mutation or independent recovery authority. Schema, dependencies, library roots and source files outside constructor-owned temporary runs remain unchanged. Actual checks and limitations belong in the implementation log; remaining M2 gates still precede M3.
