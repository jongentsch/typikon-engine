# Calendar and cycle model

## Independent calendar axes

The pack contract separates three decisions that must not be collapsed into a
single "Orthodox calendar" flag:

| Axis | Current systems | Purpose |
| --- | --- | --- |
| Fixed calendar | `gregorian`, `revised_julian`, `julian` | Projects the civil liturgical date into the month and day used to select fixed observances |
| Paschalion | `orthodox_julian` | Calculates Orthodox Pascha in the civil Gregorian year |
| Tone cycle | `octoechos` plus eight pack tone names | Calculates an ordinal from the Paschal anchor, then maps it to tradition vocabulary |

This permits a future Old Calendar pack or OCA calendar profile to use
`fixed: julian` while retaining the same Orthodox Paschalion. The existing
GOARCH and OCA fixtures use Revised Julian fixed dates.

## Date pipeline

The public request date is a proleptic Gregorian civil date. Compilation then:

1. applies the service's liturgical-day offset;
2. projects that liturgical date into the pack's fixed calendar;
3. selects fixed observances by the projected month and day;
4. calculates Orthodox Pascha for the civil year;
5. classifies the movable cycle as `triodion`, `pentecostarion`, or `ordinary`;
6. calculates the Octoechos ordinal and maps it through the pack's eight tone
   names when the ordinary tone cycle is active.

The output retains the civil liturgical date, projected fixed date, fixed
calendar, Pascha, and calculated tone. It also emits a derivation record for
each component, including inputs, method, and output. An automatically selected
fixed observance points to the fixed-date derivation.

Civil dates and Pascha use JSON Schema's Gregorian `date` format. The projected
fixed date uses a calendar-neutral `YYYY-MM-DD` shape instead, because a valid
Julian leap day such as `2100-02-29` is not a valid Gregorian date.

## Implemented arithmetic

`julian` projection converts through an absolute Julian day number rather than
assuming a permanent thirteen-day displacement. Tests cover the contemporary
thirteen-day relation and the fourteen-day relation after 2100.

`revised_julian` uses Milanković's 900-year leap cycle: non-century years
divisible by four are leap years, while century years are leap only when their
remainder modulo 900 is 200 or 600. The implementation is anchored where the
Revised Julian and Gregorian calendars coincide and supports civil years 1600
through 9999. A test covers their first future divergence in 2800.

`orthodox_julian` uses the traditional Julian ecclesiastical computus and
converts the resulting Julian date to the civil Gregorian calendar. Published
OCA Pascha dates from 2023 through 2030 are regression cases.

The Triodion phase begins 70 days before Pascha, on the Sunday of the Publican
and Pharisee, and continues through Holy Saturday. The Pentecostarion phase
runs from Pascha through the Sunday of All Saints, 56 days later. All remaining
dates are classified as ordinary.

The `octoechos` calculation anchors Tone 1 on the Sunday after Pascha. The OCA
description restarts Tone 1 on the second Sunday after Pentecost, exactly eight
weeks later, so both descriptions produce the same ordinal for ordinary-cycle
dates. Pascha through Bright Saturday deliberately returns a `null` ordinary
Octoechos tone with a suspension derivation rather than failing calendar
context construction or inventing a tone.
The four dated tradition witnesses independently confirm the calculated tones.

## Current boundary

The three-way phase identifies the governing movable-cycle book; it does not
yet model the many internal subperiods. Feast interaction, transfer, and
precedence calculations remain future work. Supplied CLI `--tone` and `--phase`
values are treated only as assertions and fail if they disagree with the
calculation.

## Evidence reviewed

- GOARCH, dating Pascha and the continued Julian basis of the Orthodox
  calculation:
  <https://www.goarch.org/-/dating-pascha-in-the-orthodox-church>
- OCA, published Paschal-cycle dates:
  <https://www.oca.org/fs/paschal-cycle>
- OCA, the Triodion beginning on the tenth Sunday before Pascha:
  <https://www.oca.org/news/archived/resources-the-pre-lenten-sundays-preparing-for-our-lenten-journey>
- OCA, the Pentecostarion beginning at Pascha and ending with the Sunday of All
  Saints:
  <https://www.oca.org/reflections/fr.-steven-kostoff/called-by-the-saints>
- GOARCH, the recurring eight-week Octoechos sequence beginning after Pascha:
  <https://www.goarch.org/-/orthodox-worship>
- OCA, the eight-week sequence and its second-Sunday-after-Pentecost anchor:
  <https://www.oca.org/wonder/the-weekly-cycle>
- OCA Holy Synod, a concrete Revised Julian / Julian fixed-feast relation:
  <https://www.oca.org/news/headline-news/announcement-regarding-matushka-olgas-canonization-services-and-feast-day>
- Dimitrijević and Theodossiou, mathematical description of the Revised Julian
  calendar:
  <https://images.astronet.ru/pubd/2008/09/28/0001230709/145-147.pdf>
