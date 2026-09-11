# Radios and antennas

A radio's datasheet stops at its antenna pin. Everything after it, the pigtail, the
feed line, the connectors, the antenna, the mast, and the path, adds or takes away
decibels, and a regulator caps how much power may leave the antenna at all. This page
covers those parts in the plain terms the guides use: what each one does to a link,
how to test it, how to mount it, and what it may radiate. The figures come from the
manufacturers' documents and the standards linked in each section, and the parts have
cards on the [Hardware](hardware.md#antennas-cables-and-protection) page.

The whole page is one sum. The power a receiver sees is the transmit power, plus the
gain of each antenna, less the loss in every cable and connector, less the path. The
link closes while that stays above the receiver's sensitivity, and what is left over
is the margin. Each section below is one term of that sum, and the
[LoRa guide](guides/lora.md) works the same sum out in code.

## Radios and concentrators

A node carries one transceiver, which hears one channel at a time: the SX1276, the
SX1262, or the LLCC68, which Semtech makes pin-to-pin compatible with the SX1262 for
medium-range indoor and indoor-to-outdoor links. A gateway carries a concentrator
instead, a baseband chip that listens on many channels and spreading factors at once
behind separate RF front ends. Semtech's SX1302 works with SX1250 front ends, which
cover 150 to 960 MHz, and its Corecell reference design receives eight 125 kHz LoRa
channels at SF5 to SF12 simultaneously, plus one LoRa channel at 250 or 500 kHz and
one FSK channel. The SX1303 is size and pin compatible with the SX1302 and adds a fine
timestamp, which lets a network locate a node by time difference of arrival.

The RAK2287, the RAK5146, and the Seeed WM1302 put those chips on mini PCIe cards
with a U.FL antenna connector. The RAK5146 adds an SX126x radio for listen before
talk, and Semtech's `sx1302_hal` software uses the additional SX1261 on the Corecell
reference design to scan the spectrum.

What to get right: a concentrator decodes what reaches it, so everything below still
applies to the gateway's antenna and feed line, and a USB concentrator talks to its
host through an STM32 bridge rather than bare SPI.

pamoja: [`pamoja-lora`](guides/lora.md) works out the airtime, the channel plans,
and the link budget for all of these radios. Sources: the
[SX1302](hardware.md#sx1302), [SX1303](hardware.md#sx1303),
[SX1250](hardware.md#sx1250), [LLCC68](hardware.md#llcc68), and
[Corecell](hardware.md#corecell) cards, and the
[`sx1302_hal` readme](https://github.com/Lora-net/sx1302_hal/blob/master/readme.md).

## Antennas

An antenna adds no power. It sends what the radio gives it in some directions more than
others, and its gain is how much stronger it is in its best direction than a reference.
Against an isotropic reference, which radiates equally in every direction, the unit is
dBi; against a half-wave dipole it is dBd. A dipole has a gain of 2.15 dB over
isotropic, so dBi = dBd + 2.15, and a gain quoted without its unit cannot be compared.

The common shapes, as TE Connectivity's application note AN-00501 describes them:

- A dipole radiates a doughnut around its axis.
- A quarter-wave monopole on a ground plane is that doughnut cut in half along its
  equator and set on the plane, with the antenna standing up through the middle.
- A directional antenna such as a Yagi narrows the pattern into a beam.

Directivity is how much stronger the pattern's peak is than the average in every
direction, and gain is directivity once the antenna's losses are counted. The higher
the directivity, the narrower the pattern, so an antenna with more gain is stronger
where it points and weaker elsewhere. Published figures make the spread concrete. TE's
ANT-868-CW-HWR, a center-fed half-wave whip, has a peak gain of -2.3 dBi at 868 MHz,
and its 916 MHz sibling 1.2 dBi. The RAKwireless fiberglass antennas for 863 to
870 MHz and 902 to 928 MHz each state 5 dBi, vertical polarization, and a 360 degree
beamwidth.

Every antenna is built for a band. Its VSWR, covered below, rises away from the
frequency it was tuned for, and the range over which it stays within the specified
VSWR is what the datasheet calls its bandwidth.

What to get right: compare gains in the same unit, choose the pattern for where the
other end of the link will be, and use an antenna tuned for the band in use.

pamoja: an antenna is `transmit_antenna_gain_dbi` or `receive_antenna_gain_dbi` in a
[`LinkBudget`](guides/lora.md), always in dBi. Sources:
[TE AN-00501](https://www.te.com/content/dam/te-com/documents/appliances/global/AN-00501%20-%20Understanding%20Antenna%20Specifications%20and%20Operation%20-%208-20-12.pdf),
the [ANT-868-CW-HWR](https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=ANT-868-CW-HWR-ccc&DocType=Data+Sheet&DocLang=English&DocFormat=pdf&PartCntxt=ANT-868-CW-HWR-SMA)
and [ANT-916-CW-HWR](https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=ANT-916-CW-HWR-ccc&DocType=Data+Sheet&DocLang=English&DocFormat=pdf&PartCntxt=ANT-916-CW-HWR-SMA)
datasheets, and the [863 to 870 MHz](hardware.md#rakarg18) and
[902 to 928 MHz](hardware.md#rakarg19) antenna cards.

## Ground planes and enclosures

A monopole's other half is whatever it is referenced to: a copper fill on the circuit
board, or a metal enclosure. AN-00501 shows how much that counterpoise matters. One
with a radius of a wavelength or more behaves close to an infinite plane. Below a
wavelength the pattern and the input impedance suffer, and at a quarter wavelength or
less the performance drops significantly. At 868.1 MHz a wavelength is 345 mm and a
quarter wavelength 86 mm; at 915 MHz they are 328 mm and 82 mm.

The plane also retunes the antenna. In the note's measurements, a 916 MHz antenna tuned
on a 4 by 4 inch plane resonated lower, with a wider bandwidth, once it stood on a
full-wave plane. An antenna tuned on the large plane and moved to the small one shifts
higher and narrows instead, which can push its VSWR out of specification at the
frequency in use. Helical antennas, coiled to save length, start with a narrow
bandwidth and suffer most.

A center-fed half-wave dipole carries its own counterpoise, which is why TE's HWR whips
need no external ground plane. For a monopole or a helical, match the board or
enclosure to the manufacturer's reference plane, and measure the antenna where it will
live rather than on the bench.

What to get right: the size of the ground plane against the antenna maker's reference,
and a measurement in the final enclosure.

Sources: [TE AN-00501](https://www.te.com/content/dam/te-com/documents/appliances/global/AN-00501%20-%20Understanding%20Antenna%20Specifications%20and%20Operation%20-%208-20-12.pdf)
and the [ANT-916-CW-HWR](https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=ANT-916-CW-HWR-ccc&DocType=Data+Sheet&DocLang=English&DocFormat=pdf&PartCntxt=ANT-916-CW-HWR-SMA)
datasheet.

## Connectors and pigtails

Connector names describe two things that are easy to mix up. The housing is a plug or
a jack, and the center conductor is a pin (male) or a socket (female). An SMA plug has a
pin and an SMA jack a socket. A reverse-polarity SMA, RP-SMA, swaps the center
conductors and keeps the housings, so an RP-SMA plug has a socket and an RP-SMA jack a
pin. An SMA plug threads onto an RP-SMA jack, but two pins meet, and an RP-SMA plug on
an SMA jack puts two sockets together, so neither pairing carries a signal. TE's
application note AN-00601 lays the naming out and notes that U.FL connectors, also sold
as MHF and UMCC, often follow the RP-SMA convention.

U.FL is the small snap-on connector on most radio modules and concentrator cards.
Hirose's U.FL series is 50 ohms with a VSWR of 1.2 or 1.3 at most, depending on the
part, and some parts are rated for as few as 30 mating cycles, so a U.FL joint is not
one to plug and unplug on the bench.

A pigtail carries the U.FL out to a bulkhead SMA through thin coax, which loses far
more per meter than any feed line below. TE's drawing for its CSI-SGFE U.FL to SMA
assemblies on 1.13 mm cable gives the typical insertion loss as a constant times the
square root of the frequency in gigahertz:

| Length | Insertion loss | At 868 MHz | At 915 MHz |
| --- | --- | --- | --- |
| 100 mm | 0.34 √f dB | 0.32 dB | 0.33 dB |
| 200 mm | 0.56 √f dB | 0.52 dB | 0.54 dB |
| 300 mm | 0.73 √f dB | 0.68 dB | 0.70 dB |

What to get right: SMA to SMA and RP-SMA to RP-SMA, the shortest pigtail that reaches,
and a U.FL joint fitted once.

pamoja: a pigtail is part of `transmit_cable_loss_db` or `receive_cable_loss_db`,
added to the feed line it connects to. Sources:
[TE AN-00601](https://www.te.com/content/dam/te-com/documents/appliances/global/AN-00601%20RF%20Coaxial%20Connector%20Gender%20Naming.pdf),
the [Hirose U.FL series](https://www.hirose.com/en/product/series/U.FL), the
[TE CSI-SGFE drawing](https://www.te.com/commerce/DocumentDelivery/DDEController?Action=showdoc&DocId=Customer+Drawing%7FC-CSI-SGFE-ccc-UFFR-p%7FC%7Fpdf%7FEnglish%7FENG_CD_C-CSI-SGFE-ccc-UFFR-p_C.pdf%7FCSI-SGFE-100-UFFR),
and the [MHF to SMA pigtail](hardware.md#rak-mhf-sma-pigtail) card.

## Feed line

Coaxial loss grows with length and with frequency, and a gateway's feed line costs its
loss twice: once on what it hears and once on what it sends. Times Microwave publishes
the attenuation of its LMR cable; its 900 MHz column, the nearest to the LoRa bands,
reads:

| Cable | Loss per 100 m | Loss per 10 m |
| --- | --- | --- |
| LMR-195 | 36.5 dB | 3.65 dB |
| LMR-240 | 24.8 dB | 2.48 dB |
| LMR-400 | 12.8 dB | 1.28 dB |
| LMR-600 | 8.2 dB | 0.82 dB |

The loss is lower at 450 MHz than at 900 MHz in every row, so the 900 MHz figures are an
upper bound for the 868 MHz band. Take the node and gateway from the LoRa guide: a node
radiating 15.65 dBm, a 6 dBi gateway antenna, and a gateway that hears down to
-140.03 dBm. Before any feed line, that link survives 161.68 dB of path loss. Ten meters
of LMR-195 between the gateway and its antenna leaves 158.03 dB; ten meters of LMR-400
leaves 160.40 dB.

What to get right: the thickest cable the run allows, and the shortest run, since every
meter at the gateway comes off both directions.

pamoja: the feed line is `receive_cable_loss_db` at the gateway, or
`transmit_cable_loss_db` when the gateway sends. Sources:
[Times Microwave LMR brochure](https://timesmicrowave.com/wp-content/uploads/2022/06/lmr-brochure-r.pdf)
and the [LMR loss](hardware.md#lmr-loss) card.

## Matching and VSWR

A radio, its cable, and its antenna are all built for 50 ohms, and wherever one of them
is not, part of the power reflects back toward the radio. The voltage standing wave
ratio measures that. A VSWR of 1:1 is a perfect match, and AN-00501 takes 2:1 as the
usual benchmark, at which 88.9% of the power sent to the antenna is radiated. The
reflection coefficient is (VSWR - 1) / (VSWR + 1), the share of the power reflected is
its square, and return loss and mismatch loss follow from those two:

| VSWR | Reflected | Delivered | Return loss | Mismatch loss |
| --- | --- | --- | --- | --- |
| 1.2:1 | 0.83% | 99.2% | 20.83 dB | 0.04 dB |
| 1.5:1 | 4.00% | 96.0% | 13.98 dB | 0.18 dB |
| 2.0:1 | 11.11% | 88.9% | 9.54 dB | 0.51 dB |
| 3.0:1 | 25.00% | 75.0% | 6.02 dB | 1.25 dB |

A 2:1 match costs half a decibel, about what 1.4 m of LMR-195 costs at 900 MHz.

A vector network analyzer measures this directly. Calibrate it with open, short, and
load standards at the connector the antenna will attach to, so the cable and adapters
before that point drop out of the reading, then sweep the band and read the VSWR or the
return loss at the channels in use. Measure the antenna mounted as it will be deployed:
in its enclosure, on its board or mast, near the metal it will sit beside. As the ground
plane measurements show, the surroundings move the resonance, and a sweep on the bench
describes the bench.

What to get right: calibration at the antenna's own connector, and a sweep of the
antenna in place.

pamoja: mismatch loss belongs in the cable loss of the end it occurs at. Sources:
[TE AN-00501](https://www.te.com/content/dam/te-com/documents/appliances/global/AN-00501%20-%20Understanding%20Antenna%20Specifications%20and%20Operation%20-%208-20-12.pdf)
and the [Times Microwave LMR brochure](https://timesmicrowave.com/wp-content/uploads/2022/06/lmr-brochure-r.pdf).

## Comparing antennas in the field

A VSWR sweep shows an antenna accepts the power; only the path shows whether it helps.
Compare two antennas on the same node at the same spot, swapping them back and forth
rather than testing one in the morning and the other in the afternoon, and let the
gateway record the RSSI and SNR of every packet. Send enough packets on each to see how
much a fixed link varies from packet to packet, and trust a difference in the medians
only when it is larger than that spread. Then repeat the comparison from a second spot,
because an antenna that wins toward one gateway can lose toward another.

What to get right: alternating antennas, many packets each, and more than one location.

pamoja: the measured RSSI less the receiver's sensitivity is the margin the link really
has, which [`LinkBudget::margin_db`](guides/lora.md) predicts from the parts.

## Height and clearance

Free-space loss assumes nothing is in the way. ITU-R P.526-16 treats a path as line of
sight, with negligible diffraction, when no obstacle enters the first Fresnel ellipsoid
around the straight line between the antennas, and it starts the diffraction zone where
the clearance falls to 60% of that ellipsoid's radius. The radius is widest halfway
along the path:

| Path at 868.1 MHz | Radius halfway | 60% of the radius |
| --- | --- | --- |
| 5 km | 20.8 m | 12.5 m |
| 15 km | 36.0 m | 21.6 m |

A roofline, a tree line, or a rise in the ground that reaches into that radius costs
diffraction loss on top of the free-space figure. Raising either antenna lifts the line
and the ellipsoid around it.

What to get right: the clearance halfway along the path, not just at each end.

pamoja: `fresnel_radius_mm` and `free_space_loss_db` in the
[link budget](guides/lora.md). Sources:
[ITU-R P.526-16](https://www.itu.int/rec/R-REC-P.526-16-202511-I/en) and
[ITU-R P.525-5](https://www.itu.int/rec/R-REC-P.525-5-202411-I/en).

## Lightning, bonding, and weather

An outdoor antenna is a path to earth for lightning, and ITU-T K.71 sets out how a
customer antenna installation is protected. Whether it needs protection against a direct
strike comes from a risk assessment under IEC 62305-2; where it does, the lightning
protection system follows IEC 62305-3 or a national standard. When bonding is required,
K.71 connects the antenna mast to earth through a bonding conductor installed straight
and vertical, by the shortest and most direct path, and connects the outer conductor of
every coaxial cable from the antenna to the mast or to the building's earthing system.
Loops are avoided, the materials must not corrode against each other, and surge
protective devices are chosen for the overcurrent expected where they are installed.

RAKwireless recommends a lightning arrestor on every N-type antenna terminal of a
gateway. The one it sells is rated for 10 kA of nominal and 20 kA of maximum discharge
current, a voltage protection level of 1200 V or less, and 0.2 dB of loss or less up to
2000 MHz, which a budget carries as one more connector.

Keep water out of the feed line and its connectors. Times Microwave makes LMR in a
watertight version, suffixed DB, flooded with a compound that stops water migrating
along the cable, and sells tape and cold-shrink kits for sealing connectors. The
RAKwireless fiberglass antennas mold the connector into the body and are rated IP67.

What to get right: the risk assessment first, then a straight bond from the mast to
earth, the cable shield bonded, and a sealed connector at every joint outdoors.

Sources: [ITU-T K.71](https://www.itu.int/rec/T-REC-K.71-201106-I/en), the
[lightning arrestor](hardware.md#rak-lightning-arrestor) card, and the
[Times Microwave LMR brochure](https://timesmicrowave.com/wp-content/uploads/2022/06/lmr-brochure-r.pdf).

## Power limits and duty cycle

A regulator limits what leaves the antenna, so the antenna's gain decides how much power
the radio may use. The limits here are quoted from the documents themselves. National
rules put them into force, and a deployment checks its own country's.

### Europe

CEPT's ERC Recommendation 70-03 sets out the bands for non-specific short range devices,
and ETSI EN 300 220-2 is the Harmonised Standard that equipment is tested against. For the
band LoRaWAN uses in Europe the two agree:

| Band | ETSI | ERC | Maximum e.r.p. | Access | Occupied bandwidth |
| --- | --- | --- | --- | --- | --- |
| 863 to 865 MHz | K | h1.3 | 25 mW | 0.1% duty cycle or polite spectrum access | 2 MHz |
| 865 to 868 MHz | L | h1.4 | 25 mW | 1% duty cycle or polite spectrum access | 3 MHz |
| 868.0 to 868.6 MHz | M | h1.5 | 25 mW | 1% duty cycle or polite spectrum access | 600 kHz |
| 868.7 to 869.2 MHz | N | h1.6 | 25 mW | 0.1% duty cycle or polite spectrum access | 500 kHz |
| 869.4 to 869.65 MHz | O | h1.7 | 500 mW | 10% duty cycle or polite spectrum access | 250 kHz |
| 869.7 to 870 MHz | P | h1.8 | 5 mW | no requirement | 300 kHz |
| 869.7 to 870 MHz | Q | h1.9 | 25 mW | 1% duty cycle or polite spectrum access | 300 kHz |

The bandwidth column is ETSI's; ERC 70-03 leaves it unspecified and writes the
alternative to a duty cycle as LBT+AFA. ETSI defines polite spectrum access as
techniques that employ clear channel assessment, and the duty cycle as the share of an
observation interval spent transmitting, which is one hour over the permitted band
unless a row says otherwise. ERC 70-03 also warns that not every entry is available in
every CEPT country; its implementation tables list each one.

Both documents state power as effective radiated power, e.r.p., which ETSI defines
against a half-wave dipole. A dipole has 2.15 dB of gain over isotropic, and ETSI EN
300 220-1 writes 0 dB relative to a dipole as +2.15 dBi, so the e.i.r.p. of the same
transmitter is 2.15 dB higher. 25 mW e.r.p. is 13.98 dBm e.r.p. and 16.13 dBm e.i.r.p.,
and 500 mW e.r.p. is 29.14 dBm e.i.r.p. The EU863-870 plan in `pamoja-lora` follows the
LoRaWAN regional parameters and states its ceilings as EIRP, 16 dBm in the sub-band that
holds 868.1 MHz.

Above 870 MHz, row h3 of ERC 70-03 designates 915 to 919.4 MHz at 25 mW e.r.p. with a 1%
duty cycle, and 100 mW e.r.p. in the RFID channels centered on 916.3, 917.5, and
918.7 MHz. Where extended GSM-R needs protection, 918 to 919.4 MHz is held to a 0.01%
duty cycle and 5 ms of transmission in any second.

### United States

47 CFR 15.247 caps the peak conducted output power in 902 to 928 MHz, the power at the
antenna connector rather than what the antenna radiates. A system using digital
modulation, whose 6 dB bandwidth must be at least 500 kHz, may use 1 W. A frequency
hopping system may use 1 W on at least 50 hopping channels, and 0.25 W on 25 to 49, which
the rule allows only when each hopping channel's 20 dB bandwidth is 250 kHz or more; a
hopping system also stays no longer than 0.4 seconds on any one frequency within 20
seconds, or within 10 seconds for the wider channels. The caps assume a transmitting
antenna of no more than 6 dBi. Past that, the conducted power comes down by every
decibel the gain exceeds 6 dBi, so a 9 dBi Yagi on a 1 W system leaves 27 dBm at the
connector.

What to get right: which limit applies, conducted or radiated, e.r.p. or e.i.r.p.; the
antenna gain taken off before the radio's setting; and the duty cycle counted over the
whole hour.

pamoja: `LinkBudget::max_transmit_power_dbm` takes an EIRP ceiling back to the radio's
setting, and `Fcc15247` applies the United States rule, both in the
[link budget](guides/lora.md). Sources:
[ERC Recommendation 70-03](https://docdb.cept.org/download/4635),
[ETSI EN 300 220-2 V3.3.1](https://www.etsi.org/deliver/etsi_en/300200_300299/30022002/03.03.01_60/en_30022002v030301p.pdf),
[ETSI EN 300 220-1 V3.1.1](https://www.etsi.org/deliver/etsi_en/300200_300299/30022001/03.01.01_60/en_30022001v030101p.pdf),
and [47 CFR 15.247](https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-15/subpart-C/section-15.247).
