# Native Feast director investigation

## Native party row contract, June 7.51h2

Current candidate enables native `PvPMKSPartyList1` / `PvPMKSPartyList3`
RequestedUpdate and drawing. The CC producer remains bypassed. Both use
vtable VA142251100 (variant142251378), RequestedUpdate RVA13C5180,
row renderer13C5930, ReceiveEvent13C5240. Array117/105 sizes must be >=602/130.
Team t=0/1: b=10+t*296, display order at b+11..15. Row r starts at
n=b+16+r*56; strings start at s=t*65+r*12. Five native slots, four Feast players.
Number offsets: present0, actorID1, alive2, name/outline UIColor3/4, jobicon5,
HP7/8, shield9, MP10/11, adrenaline12, castprogress13, iconcount15,
iconIDs36..45, timeractive46..55. Strings: name s+5, cast s+6, timers s+7..16.
Unused fields are cleared. Archived additional action icon IDs are in
target/feast-whm/Action-6.08.csv. Status tooltip IDs remain zero for action icons.

Native agent ReceiveEvent RVAD5BDA0 compares clicked team against number[3].
Equal teams take regular-party-slot selection; unequal teams use D5B8F0 to
resolve the actor from this same array, AF8090 object lookup, 626AD0 validation,
629990 target selection. Setting number[3]=-1 (native reset sentinel) selects
the actor path for both lists. Number8/9 carry current hard/soft target IDs.
No replacement mouse hitboxes or managed click handlers are installed.

F4 continuation: tag, actor u32 LE, slot u8 (0..2), historical action u16 LE,
remaining tenths u16 LE. Sent after that actor's F2 and before F3 commit.
Parser rejects unknown owners/actions, duplicate slots, or cooldown >45s;
cooldowns are committed atomically with the roster. Lua exports deadlines for
Purify plus chosen additional actions, and does not change gameplay while read.
44 world tests and 24 client protocol checks pass; live validation is pending.

## Native presentation candidate, 2026-09-15

User explicitly approved using the existing Kawari.FeastHud Dalamud module to
replace the client presentation. Server gameplay remains authoritative.
The candidate now replaces verified PvPHeader refresh RVA `D5C0A0`, taking
`(agent, director, context)`. It checks territory 745 and legacy vtable `234E100`.
The retail routine dereferences director offsets `212C`, `2138`, and the CC tail
passed to helpers at `D5A4D0`, `D5CB50`, `D5CDF0`, `D5A370`; the legacy allocation
ends at `20F8`. Header opening at `D5C670` is retained. Its standard agent virtual
Update is empty, so hooking that virtual function would not fix this bug.
The native unbinding/progress producer is bypassed along with these invalid reads.

Client compatibility guard: executable SHA-256
`4236E770E673150E85F8D10BEAB2FC4834C82F86AAB8A555A9175439FC906A6D`, plus entry
prefix checks for both native functions. SDK signatures resolve SetDirectorData
and action-status hooks. No executable or installed asset mutation in this step.

DirectorVars handler `0x80039C56`, flag/branch zero, carries frames every 250 ms:

| Tag | Ten-byte data layout |
| --- | --- |
| F1 | tag, phase/team byte, seconds u16, Fang u16, Claw u16, personal u8, respawn seconds u8 |
| F2 | tag, team u8, actor u32, medals u16, job u8, alive u8 |
| F3 | tag, ground medals u16, seven zero bytes; commits the preceding frame |

Integers are little-endian. Phase is bits 0–2 (prep/active/sudden/Fang/Claw/draw),
team is bits 3–4 (spectator/Fang/Claw). Present participants are sent between F1
and F3; publishing only at commit removes departed players and avoids partial UI
updates. The module intercepts F2/F3; F1 is also copied into native UnionData.
The existing normal duty InitDirector and exit lifecycle remain unchanged.

Native addon lifecycle listeners suppress requested CC updates and attach owned
text nodes to the native header and two party addons. Original ULD children are
hidden, restored before finalization; owned nodes are unlinked and destroyed.
This first candidate is native text, not the recovered historical Feast artwork
or complete party HP/status/target controls. WHM PvP hotbars receive temporary
Feast placeholder actions with recovery; native ActionRequest routing is retained.
Retail action-status checks are bypassed only for implemented WHM actions here.
Other jobs and controller cross hotbars still need implementation.

Validation: all 44 world tests, 19 client protocol/framing checks, Release HUD
build and Oodle world build pass. User closed the client for deployment; live
rendering, notification suppression and action acceptance are still unconfirmed.

Client inspected: installed Windows 7.51h2, `2026.06.18.0000.0000`.
Investigation date: 2026-09-15. Reads included installed Excel tables, executable
code, and the running client's registered factory map. No client code was executed
by the inspection tools and no installed metadata was changed.

## Decision

A legacy PvP director implementation survives. Test and reuse it before extending
UI workarounds. Do not substitute Crystalline Conflict: its factory path differs.
Native migration is not yet validated. Probe activation is disabled following the
failed live test below. Use the normal duty lifecycle for the next experiment.

## Current baseline and recovered duty identities

The user confirmed normal login and Sastasha entry with enemies present. Duty Finder entries
were initially greyed out. `!unlock all` does not
unlock duties: use `!unlockcontent all`, then log out and back in. If necessary,
`!feast baseline` in an open-world area queues current CFC 4 (Sastasha,
`s1d1_re`) through `register_for_content`. Accepting the prompt uses the ordinary
`JoinContent` path. This Lua command reloads on use and needs no server restart.
Logs confirmed InitDirector for InstanceContent 4 and a successful entrance-event
return at 2026-09-15T10:28:14Z / 10:28:23Z. Stone action 127 had no server script;
that is a separate combat limitation, not evidence of director entry failure.

## Installed duty-entry candidate

Installed at 2026-09-15T10:39Z with user authorization after they closed the client.
All eight services restarted successfully; world ready at 10:40:01Z, stderr empty.
The first native Feast entry/exit test completed. **Native presentation is incompatible.**

### Live outcome

- User entered territory 745 through duty acceptance and was visible. Server logged
  InitDirector 40022 at 10:42:19Z. ReadProcessMemory confirmed the active director's
  vtable RVA **0x234E100**, the legacy class identified above, not the CC constructor.
- Client showed CC hotbars, Astra/Umbra header, invalid negative percentages, empty
  team panels, and repeated `Unbinding in 0s...`. Existing plugin Feast HUD separately
  showed the authoritative server scores. These are not native Feast score updates.
- `!feast leave` returned to New Gridania. User confirmed all PvP panels and the
  repeated alert cleared and the normal hotbar returned.
- Initial town entry was rejected because the command guard allowed only intended
  use 1. Fixed live: Town is 0, OpenWorld is 1. Lua checks now cover both, rejection
  from a duty, exploration fallback and both leave paths.

The executable's getter at RVA **0xB80AF0** accepts handler 0x8003 and byte
`director+0xDCE == 5`, without checking the content ID or concrete class. That is
consistent with legacy allocation coexisting with newer PvP consumers; the exact
header/notification caller chain still needs tracing. The screenshot does **not**
prove the alert is caused by a missing crystal. Do not spawn a crystal to mask it.

`PvPColosseumHeader` and the two old party-list names survive in a name/id/flags
table at RVA 0x21EF670 (IDs 73/74/75), beside job-HUD and current CC entries.
This is not evidence of a surviving addon constructor. The old header string has
one absolute-pointer reference to this table; no direct RIP-relative LEA reference
was found. Current `PvPMKSHeader` additionally has a direct code reference at RVA
0x13C3126. More investigation is needed before claiming old UI can be selected by
restoring layouts alone.

Read-only research tools: target/feast_type5_getter.py, feast_getter_callers.py,
feast_header_refs.py, feast_header_tables.py, feast_header_record_table.py.
Addon row 14494 contains the unbinding text. An immediate-byte search produced a
coincidental call displacement, not a verified notification reference; do not use
that match as evidence.

- `prepare_feast_duty.py` prepares current-schema fields and verifies original
  state, edited field values, unrelated-row preservation, and archive round trips.
- CFC 476 points to territory 745 / content 40022 / `l1p2`, in all four languages.
  InstanceContent 40022 has type 5, six-minute duration and ReqInstance 40001.
  Territory 745 now has intended use 28 (the current enum calls it
  CrystallineConflict) and the PvP flag. The legacy content ID selects the old
  factory branch; this does not establish compatibility with downstream CC code.
- Server entry validates all three linked definitions before queue and acceptance.
  `!feast` chooses the duty path when metadata is ready; `!feast duty` explicitly
  requests it. In the exploration metadata state the original entry still works.
- `content/l1p2.lua` creates the server director through the normal lifecycle.
  It deliberately assigns no guessed score/state bytes. Match rules remain in
  `server/feast_match.rs`; native match-state updates are not implemented yet.
- Duty entry stages the player at the known west-jail spawn because Feast lacks a
  dungeon entrance circle. `!feast leave` uses AbandonContent and the saved return
  location in duty mode.
- The old `!feast director on` attach-after-teleport probe remains disabled.

Backups and installation log: `backups/feast-duty-7.51h2`. Installer preflights all
six allocations and rolls back touched allocations on write failure. Restore
with `tools/feast-map/Install-FeastDuty.ps1 -Restore` after closing the client,
launchers and server. This returns to the exploration patch, not the blank retail
territory. Do not use the older exploration restore while the duty patch is active.

Validation: cargo check and Oodle-enabled build pass; 42 world library tests pass;
three installer tests cover round-trip restore, preflight rejection and rollback
after a simulated write failure. Entry and exit work; native UI/hotbars have the
incompatibility above. Respawn remains unverified in duty mode.

Recovered pre-6.1 CFC rows have the following identities, decoded from the recovered
EXH and EXD (not inferred from their display names):

| CFC | Short code | Territory | InstanceContent | Mode |
| --- | --- | --- | --- | --- |
| 476 | l1p2 | 745 | 40022 | Team ranked |
| 478 | l1p2_2 | 765 | 40023 | Ranked |
| 479 | l1p2_3 | 766 | 40024 | Training |
| 480 | l1p2_4 | 767 | 40025 | Custom |
| 580 | l1p2_5 | 767 | 40026 | Team custom |

Thus territory 745's historical identity is CFC 476 / InstanceContent 40022.
Restoring custom CFC 480 verbatim would target territory 767, which our exploration
patch does not restore. Any custom-mode adaptation to 745 must explicitly reconcile
these references in the current schema. The old probe's 40001 is not 745's original
content ID.

Sastasha's lifecycle in this checkout is:

1. `register_for_content` sends the found prompt; acceptance calls `join_content`,
   which records the return location and sends `ToServer::JoinContent`.
2. `create_instance_for_content` resolves the territory, handler, short code and
   entrance. It loads content Lua and calls `onSetup` on the server director.
3. `handle_zone_change` sets duty state and sends `InitDirector` during zone load.
4. `setup_director` sends the saved `DirectorVars` and initial map effects.

The server's scripted `DirectorData` requires a successfully loaded content script.
The **client** initialization is separate: `handle_zone_change` sends InitDirector
based on intended use and content metadata even when that server script is missing;
`setup_director` then falls back to default variables. Do not mistake absence of a
server script for a guarantee that the client cannot create a director.

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

## Previous compatibility probe (activation disabled)

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
