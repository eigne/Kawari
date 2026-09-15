# Native Feast director investigation

Client inspected: installed Windows 7.51h2, `2026.06.18.0000.0000`.
Investigation date: 2026-09-15. Reads included installed Excel tables, executable
code, and the running client's registered factory map. No client code was executed
by the inspection tools and no installed metadata was changed.

## Decision

A legacy PvP director implementation survives. Test and reuse it before extending
UI workarounds. Do not substitute Crystalline Conflict: its factory path differs.
Native migration is not yet validated; the new command is an opt-in compatibility
probe, not a replacement for authoritative Feast match rules.

## Evidence

- Territory 745 currently has name `l1p2`, intended use 1, CFC 0. Our arena entry
  is an ordinary territory change. It does not create a duty instance.
- Current InstanceContent 40001 still has type 5 and references CFC 121, but that
  CFC row is blank. Former entries 40002–40026 have cleared metadata. New CC
  entries start at 40027; custom Palaistra uses 40033/CFC 862/territory 1058.
- The live DirectorModule registers handler 0x8003 with factory RVA 0x1AA4180.
  The inspected client had no active content director at the time of the read.
- This factory explicitly routes IDs 40001–40026 to allocation size 0x20F8 and
  constructor RVA 0x19AD440, via branch RVA 0x1AA7619. IDs 40027–40047 route to
  allocation size 0x23C0 and constructor RVA 0x1A3ACB0 via RVA 0x1AA764D.
  Therefore missing Excel rows do not imply missing legacy executable support.
- Legacy constructor calls RVA 0x1A378C0 and installs vtable RVA 0x234E100.
  Compared with the shared PvP vtable RVA 0x23F1C90, it overrides destructor,
  content-specific update slot 381 (RVA 0x19AC840), and slot 404
  (RVA 0x19AC9F0). Those update handlers contain implementations, not empty returns.
- The current type-5 getter checks handler 0x8003 and type byte +0xDCE == 5.
  It does not itself distinguish legacy content from CC. Compatibility tests
  must therefore check assumptions in downstream callers, not only allocation.
- Recovered pre-6.1 CFC rows include 476 (team ranked Crystal Tower), 478 (ranked),
  479 (training), 480 (custom), and 580 (team custom). Recovery uses patches through
  2022.03.01. Old rows cannot be copied directly into the current Excel schema.

## Reversible compatibility probe

In territory 745, `!feast director on` sends native InitDirector for existing
InstanceContent 40001, flags 0. This exercises the actual surviving legacy factory
without restoring sheets or changing intended use. `!feast director off` terminates
it. The connection tracks its handler so normal zone changes/respawn also terminate
it. Duplicate enable requests do not create another handler. It is never automatic.

Probe limitations: no native score/team data or director lifecycle is sent yet;
map/duty metadata remains the exploration setup. Successful allocation alone is not
proof that a fully restored duty or historical HUD works. Current server combat,
medal rules and HUD remain authoritative while testing.

Validation: 42 world library tests pass (including routing); cargo check passes.
Live initialization, native state fields, casting/hotbars, respawn, termination,
and reconnect still require the in-game probe.

## Migration sequence after a successful probe

1. Confirm the director is created, identify its class/vtable and inspect its state.
2. Decode its content-specific update protocol and connect the existing match state
   to its preparation, running, result, score, team and respawn fields.
3. Prepare backed-up current-schema CFC/InstanceContent/TerritoryType restoration
   and use Kawari's real duty creation path. Keep small teams for testing.
4. Check client PvP action restrictions/hotbar switching before adopting that metadata.
5. Remove plugin presentation workarounds only where native behavior is verified.

Kawari's server DirectorData currently supports scripted content, but it only gets
created when a matching content Lua script exists. Renaming Feast Runtime or
putting it in that struct alone would not initialize the native client director.

Local research outputs (ignored target directory): director-current-sheets.txt,
director-factory.txt, director_factory_ids.py, director-legacy-overrides.txt,
director_live_probe.py; recovered Excel inputs in feast-director-research.
Executable addresses above are preferred-base RVAs, specific to this client build.

## Live probe outcome

The user reported no visible effect, then an invisible character after relog and a
 disconnect after attempting Triple Triad Battlehall. The server process remained
alive; logs showed timeouts, without a recorded panic. This does not establish the
precise cause. Activation is now disabled at both command and connection handlers.
The server was restarted and the failure logs preserved. The next test must use
valid duty metadata and the normal JoinContent/zone initialization sequence; first
verify Sastasha as a baseline. Existing constructor code alone is insufficient
proof of a functional native Feast duty.
