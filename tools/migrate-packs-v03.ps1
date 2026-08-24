$ErrorActionPreference = 'Stop'

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$packs = @(
    @{ Name = 'typikon-goarch'; Authority = 'goarch-dcs-major-feast-source' },
    @{ Name = 'typikon-oca'; Authority = 'oca-service-structures' },
    @{ Name = 'typikon-antiochian'; Authority = 'antiochian-department-service-texts' }
)

function Write-Utf8([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText($Path, ($Content.Trim() + "`n"), [System.Text.UTF8Encoding]::new($false))
}

function Fixed([string]$Id, [string]$Name, [string]$Citation) {
@"
      - id: $Id
        name: $Name
        kind: fixed
        material:
          kind: fixed_text
          title: $Name
          citation: '$Citation'
"@
}

function Changeable([string]$Id, [string]$Name, [string]$Cardinality, [string[]]$Accepts) {
    $accepted = if ($Accepts.Count -gt 0) {
        "`n        accepts:`n" + (($Accepts | ForEach-Object { "          - $_" }) -join "`n")
    } else { '' }
@"
      - id: $Id
        name: $Name
        kind: changeable
        cardinality: $Cardinality$accepted
"@
}

function Section([string]$Id, [string]$Name, [string[]]$Components) {
    $result = @"
  - id: $Id
    name: $Name
    components:
$($Components -join "`n")
"@
    return $result + "`n"
}

function ServiceHeader([string]$Id, [string]$Name, [int]$Offset, [string]$Authority) {
    $result = @"
schema: typikon.service/v0.2

id: $Id
name: $Name
liturgical_day_offset: $Offset
authority:
  - $Authority
"@
    return $result + "`n"
}

function VespersService([string]$Id, [string]$Name, [string]$Authority) {
    (ServiceHeader $Id $Name 1 $Authority) + "sections:`n" +
    (Section 'opening' 'Opening' @(
        (Fixed 'opening_blessing' 'Opening blessing' 'Horologion: Vespers, opening blessing'),
        (Fixed 'come_let_us_worship' 'Come, let us worship' 'Horologion: invitatory'),
        (Fixed 'psalm_103' 'Psalm 103' 'Psalter: Psalm 103'),
        (Fixed 'great_litany' 'Great Litany' 'Vespers: Litany of Peace')
    )) +
    (Section 'psalmody' 'Psalter' @(
        (Changeable 'kathisma' 'Appointed kathisma' 'optional' @('psalmody')),
        (Fixed 'little_litany' 'Little Litany' 'Vespers: Little Litany')
    )) +
    (Section 'lord_i_call' 'Lord, I Call' @(
        (Fixed 'psalm_140' 'Lord, I call and lamp-lighting psalms' 'Psalter: Psalms 140, 141, 129, and 116'),
        (Changeable 'stichera' 'Stichera' 'many' @('resurrectional','festal','saint')),
        (Changeable 'glory' 'Glory doxastikon' 'optional' @('doxastikon')),
        (Changeable 'both_now' 'Both now theotokion or stavrotheotokion' 'optional' @('theotokion','dogmatikon','stavrotheotokion'))
    )) +
    (Section 'entrance' 'Entrance and Lessons' @(
        (Changeable 'entrance' 'Entrance appointment' 'optional' @('entrance')),
        (Fixed 'o_gladsome_light' 'O Gladsome Light' 'Vespers: Phos Hilaron'),
        (Changeable 'prokeimenon' 'Prokeimenon' 'one' @('prokeimenon')),
        (Changeable 'readings' 'Old Testament readings' 'many' @('old_testament_reading'))
    )) +
    (Section 'evening_prayer' 'Evening Prayer and Litanies' @(
        (Fixed 'vouchsafe_o_lord' 'Vouchsafe, O Lord' 'Vespers: evening prayer'),
        (Fixed 'litany_of_supplication' 'Litany of Supplication' 'Vespers: evening litany'),
        (Fixed 'peace_and_bowing' 'Peace and prayer at the bowing of heads' 'Vespers: prayer at the bowing of heads')
    )) +
    (Section 'aposticha' 'Aposticha' @(
        (Changeable 'stichera' 'Aposticha stichera' 'many' @('aposticha')),
        (Changeable 'glory' 'Aposticha Glory' 'optional' @('doxastikon')),
        (Changeable 'both_now' 'Aposticha Both now' 'optional' @('theotokion','stavrotheotokion')),
        (Fixed 'song_of_simeon' 'Song of Simeon' 'Luke 2:29-32'),
        (Fixed 'trisagion_prayers' 'Trisagion prayers' 'Horologion: Trisagion prayers')
    )) +
    (Section 'dismissal' 'Dismissal' @(
        (Changeable 'apolytikion' 'Apolytikion or troparia' 'many' @('apolytikion','troparion')),
        (Changeable 'kontakion' 'Kontakion' 'optional' @('kontakion')),
        (Fixed 'litany' 'Litany and blessing' 'Vespers: concluding litany and blessing'),
        (Changeable 'dismissal_commemorations' 'Dismissal commemorations' 'optional' @('dismissal_commemoration'))
    ))
}

function MatinsService([string]$Authority) {
    (ServiceHeader 'matins' 'Orthros (Matins)' 0 $Authority) + "`nsections:`n" +
    (Section 'opening' 'Opening and Six Psalms' @(
        (Fixed 'opening' 'Opening prayers' 'Horologion: Matins opening'),
        (Fixed 'six_psalms' 'Six Psalms' 'Psalter: Psalms 3, 37, 62, 87, 102, and 142'),
        (Fixed 'great_litany' 'Great Litany' 'Matins: Litany of Peace')
    )) +
    (Section 'god_is_lord' 'God is the Lord' @(
        (Changeable 'invitatory' 'God is the Lord or appointed Alleluia' 'one' @('god_is_the_lord','alleluia_invitatory')),
        (Changeable 'apolytikia' 'Apolytikia and theotokia' 'many' @('apolytikion','troparion','theotokion'))
    )) +
    (Section 'psalter' 'Psalter and Sessionals' @(
        (Changeable 'kathismata' 'Kathismata and sessional hymns' 'many' @('psalmody','kathisma','sessional_hymn')),
        (Changeable 'polyeleos' 'Polyeleos or appointed magnification' 'optional' @('polyeleos','magnification'))
    )) +
    (Section 'gospel' 'Matins Gospel' @(
        (Changeable 'anabathmoi' 'Songs of Ascents' 'optional' @('anabathmoi')),
        (Changeable 'prokeimenon' 'Prokeimenon' 'optional' @('prokeimenon')),
        (Changeable 'gospel' 'Matins Gospel' 'optional' @('gospel')),
        (Fixed 'psalm_50' 'Psalm 50' 'Psalter: Psalm 50')
    )) +
    (Section 'canon' 'Canon' @(
        (Changeable 'canon' 'Canons, odes, and katavasiae' 'many' @('canon','katavasia')),
        (Changeable 'magnificat' 'Magnificat or appointed Ninth Ode refrains' 'one' @('magnificat','ninth_ode_refrain'))
    )) +
    (Section 'conclusion' 'Lauds and Dismissal' @(
        (Changeable 'exapostilarion' 'Exapostilaria' 'many' @('exapostilarion')),
        (Changeable 'praises' 'Stichera at the Praises' 'many' @('praises')),
        (Changeable 'doxastikon' 'Glory and Both now at the Praises' 'optional' @('doxastikon','theotokion')),
        (Fixed 'great_doxology' 'Great Doxology' 'Horologion: Great Doxology'),
        (Changeable 'apolytikion' 'Concluding apolytikion' 'optional' @('apolytikion','troparion')),
        (Fixed 'litanies' 'Concluding litanies' 'Matins: concluding litanies'),
        (Changeable 'dismissal_commemorations' 'Dismissal commemorations' 'optional' @('dismissal_commemoration'))
    ))
}

function DivineLiturgyService([string]$Authority) {
    (ServiceHeader 'divine_liturgy' 'Divine Liturgy' 0 $Authority) + @"
default_form: chrysostom
forms:
  - id: chrysostom
    name: Divine Liturgy of Saint John Chrysostom
    authority:
      - $Authority
  - id: basil
    name: Divine Liturgy of Saint Basil the Great
    authority:
      - $Authority

sections:
"@ + "`n" +
    (Section 'preparation' 'Preparation and Enarxis' @(
        (Fixed 'opening_blessing' 'Opening blessing' 'Divine Liturgy: opening blessing'),
        (Fixed 'great_litany' 'Great Litany' 'Divine Liturgy: Litany of Peace'),
        (Changeable 'antiphons' 'Antiphons or Typika and Beatitudes' 'many' @('antiphon','typika','beatitudes'))
    )) +
    (Section 'little_entrance' 'Little Entrance' @(
        (Fixed 'entrance_prayer' 'Prayer of the Little Entrance' 'Divine Liturgy: Little Entrance'),
        (Changeable 'entrance_hymn' 'Entrance Hymn' 'optional' @('entrance_hymn'))
    )) +
    (Section 'hymns' 'Troparia and Trisagion' @(
        (Changeable 'troparia' 'Troparia' 'many' @('troparion','apolytikion')),
        (Changeable 'kontakia' 'Kontakia' 'many' @('kontakion')),
        (Changeable 'trisagion' 'Trisagion or appointed substitute' 'one' @('trisagion','trisagion_substitute'))
    )) +
    (Section 'readings' 'Liturgy of the Word' @(
        (Changeable 'prokeimenon' 'Prokeimenon' 'one' @('prokeimenon')),
        (Changeable 'epistle' 'Epistle' 'one' @('epistle')),
        (Changeable 'alleluia' 'Alleluia and verses' 'one' @('alleluia')),
        (Changeable 'gospel' 'Gospel' 'one' @('gospel')),
        (Fixed 'homily_position' 'Place appointed for the homily' 'Divine Liturgy: after the Gospel')
    )) +
    (Section 'faithful' 'Liturgy of the Faithful' @(
        (Fixed 'litanies' 'Litanies of the Faithful' 'Divine Liturgy: litanies before the Great Entrance'),
        (Changeable 'cherubic_hymn' 'Cherubic Hymn or appointed substitute' 'one' @('cherubic_hymn','cherubic_substitute')),
        (Fixed 'creed' 'Nicene-Constantinopolitan Creed' 'Divine Liturgy: Symbol of Faith')
    )) +
    (Section 'anaphora' 'Anaphora' @(
        (Fixed 'eucharistic_prayer' 'Eucharistic prayer' 'Divine Liturgy of Saint John Chrysostom: Anaphora'),
        (Changeable 'megalynarion' 'Megalynarion or hymn to the Theotokos' 'one' @('megalynarion','theotokion'))
    )) +
    (Section 'communion' 'Communion' @(
        (Fixed 'lords_prayer' "The Lord's Prayer" 'Matthew 6:9-13'),
        (Changeable 'communion_hymn' 'Communion Hymn' 'one' @('communion_hymn')),
        (Fixed 'communion' 'Communion of clergy and faithful' 'Divine Liturgy: Holy Communion'),
        (Changeable 'post_communion' 'Seasonal post-Communion hymn' 'optional' @('post_communion'))
    )) +
    (Section 'dismissal' 'Thanksgiving and Dismissal' @(
        (Fixed 'thanksgiving_litany' 'Thanksgiving litany and prayer behind the ambo' 'Divine Liturgy: Thanksgiving'),
        (Changeable 'dismissal_commemorations' 'Dismissal commemorations' 'optional' @('dismissal_commemoration')),
        (Fixed 'final_blessing' 'Final blessing' 'Divine Liturgy: Dismissal')
    ))
}

# Add form-specific Anaphora citations after constructing the common template.
function Add-AnaphoraForms([string]$Content) {
    $needle = "          citation: 'Divine Liturgy of Saint John Chrysostom: Anaphora'"
    $replacement = @"
          citation: 'Divine Liturgy of Saint John Chrysostom: Anaphora'
        form_material:
          chrysostom:
            kind: fixed_text
            title: Anaphora of Saint John Chrysostom
            citation: 'Divine Liturgy of Saint John Chrysostom: Anaphora'
          basil:
            kind: fixed_text
            title: Anaphora of Saint Basil the Great
            citation: 'Divine Liturgy of Saint Basil the Great: Anaphora'
"@.TrimEnd()
    $Content.Replace($needle, $replacement)
}

$majorTargets = @{
    'vespers' = @(
        'psalmody:kathisma','lord_i_call:stichera','lord_i_call:glory','lord_i_call:both_now',
        'entrance:entrance','entrance:prokeimenon','entrance:readings','aposticha:stichera',
        'aposticha:glory','aposticha:both_now','dismissal:apolytikion','dismissal:kontakion',
        'dismissal:dismissal_commemorations'
    )
    'matins' = @(
        'god_is_lord:invitatory','god_is_lord:apolytikia','psalter:kathismata','psalter:polyeleos','gospel:anabathmoi',
        'gospel:prokeimenon','gospel:gospel','canon:canon','conclusion:exapostilarion',
        'canon:magnificat','conclusion:praises','conclusion:doxastikon','conclusion:apolytikion','conclusion:dismissal_commemorations'
    )
    'divine_liturgy' = @(
        'preparation:antiphons','little_entrance:entrance_hymn','hymns:troparia','hymns:kontakia',
        'hymns:trisagion','readings:prokeimenon','readings:epistle','readings:alleluia',
        'readings:gospel','anaphora:megalynarion','communion:communion_hymn',
        'faithful:cherubic_hymn','communion:post_communion','dismissal:dismissal_commemorations'
    )
}

# A rule admits every component the observance may provide. A rank requirement
# is narrower: only components that must exist for every observance of the rank.
$majorRequired = @{
    'vespers' = @(
        'lord_i_call:stichera','entrance:readings','dismissal:apolytikion'
    )
    'matins' = @(
        'god_is_lord:apolytikia','canon:canon','conclusion:exapostilarion','conclusion:praises'
    )
    'divine_liturgy' = @(
        'preparation:antiphons','hymns:troparia','hymns:kontakia','readings:prokeimenon',
        'readings:epistle','readings:alleluia','readings:gospel','anaphora:megalynarion',
        'communion:communion_hymn'
    )
}

function MajorRule([string]$Service, [string]$Authority) {
    $id = $Service.Replace('_', '-')
    $emissions = ($majorTargets[$Service] | ForEach-Object {
        $parts = $_.Split(':')
@"
  - section: $($parts[0])
    component: $($parts[1])
    observance: true
"@
    }) -join "`n"
@"
schema: typikon.rule/v0.3

id: major-feast-$id
when:
  service: $Service
  observance:
    rank: major-feast
    properties:
      major_feast: true
emit:
$emissions
authority:
  - $Authority
"@
}

function MajorRank([string]$Authority) {
    $services = foreach ($service in @('vespers','matins','divine_liturgy')) {
        $requirements = ($majorRequired[$service] | ForEach-Object {
            $parts = $_.Split(':')
@"
      - section: $($parts[0])
        component: $($parts[1])
"@
        }) -join "`n"
@"
  $service`:
    required:
$requirements
"@
    }
@"
schema: typikon.rank/v0.1

id: major-feast
name: Major feast
authority:
  - $Authority
services:
$($services -join "`n")
"@
}

function SimpleRank([string]$Id, [string]$Name, [string]$Authority, [bool]$Glory) {
    $gloryRequirement = if ($Glory) { "`n      - section: lord_i_call`n        component: glory" } else { '' }
@"
schema: typikon.rank/v0.1

id: $Id
name: $Name
authority:
  - $Authority
services:
  great_vespers:
    required:
      - section: lord_i_call
        component: stichera$gloryRequirement
"@
}

foreach ($packInfo in $packs) {
    $root = Join-Path $workspace $packInfo.Name
    if ((Split-Path $root -Leaf) -notin @('typikon-goarch','typikon-oca','typikon-antiochian')) { throw "Unexpected pack root $root" }
    $authority = $packInfo.Authority

    $manifest = Get-Content (Join-Path $root 'pack.yaml') -Raw
    $manifest = $manifest.Replace('typikon.pack/v0.2', 'typikon.pack/v0.3')
    $manifest = $manifest.Replace('version: 0.1.0', 'version: 0.2.0')
    $manifest = $manifest.Replace('  resources: resources/', '  ranks: ranks/')
    Write-Utf8 (Join-Path $root 'pack.yaml') $manifest

    $legacyResources = Join-Path $root 'resources'
    $archiveResources = Join-Path $root 'archive\legacy-resource-bundles'
    if ((Test-Path -LiteralPath $legacyResources) -and -not (Test-Path -LiteralPath $archiveResources)) {
        [System.IO.Directory]::CreateDirectory((Split-Path $archiveResources -Parent)) | Out-Null
        Move-Item -LiteralPath $legacyResources -Destination $archiveResources
    }

    $services = Join-Path $root 'services'
    Write-Utf8 (Join-Path $services 'vespers.yaml') (VespersService 'vespers' 'Vespers' $authority)
    Write-Utf8 (Join-Path $services 'great-vespers.yaml') (VespersService 'great_vespers' 'Great Vespers' $authority)
    Write-Utf8 (Join-Path $services 'matins.yaml') (MatinsService $authority)
    Write-Utf8 (Join-Path $services 'divine-liturgy.yaml') (Add-AnaphoraForms (DivineLiturgyService $authority))

    $observanceFiles = Get-ChildItem (Join-Path $root 'observances') -Recurse -File -Filter '*.yaml'
    foreach ($file in $observanceFiles) {
        $content = Get-Content $file.FullName -Raw
        $content = $content.Replace('typikon.observance/v0.3', 'typikon.observance/v0.4')
        $content = [regex]::Replace($content, '(?ms)^appointments:\r?\n.*?(?=^properties:)', '')
        Write-Utf8 $file.FullName $content
    }

    $rules = Join-Path $root 'rules'
    foreach ($service in @('vespers','matins','divine_liturgy')) {
        Write-Utf8 (Join-Path $rules ("major-feast-" + $service.Replace('_','-') + '.yaml')) (MajorRule $service $authority)
    }

    $rankRoot = Join-Path $root 'ranks'
    [System.IO.Directory]::CreateDirectory($rankRoot) | Out-Null
    Write-Utf8 (Join-Path $rankRoot 'major-feast.yaml') (MajorRank $authority)

    if ($packInfo.Name -eq 'typikon-goarch') {
        Write-Utf8 (Join-Path $rankRoot 'six-stichera.yaml') (SimpleRank 'six-stichera' 'Six-stichera saint' $authority $true)
    }
    if ($packInfo.Name -eq 'typikon-oca') {
        Write-Utf8 (Join-Path $rankRoot 'six-stichera.yaml') (SimpleRank 'six-stichera' 'Six-stichera commemoration' $authority $true)
        Write-Utf8 (Join-Path $rankRoot 'lesser.yaml') (SimpleRank 'lesser' 'Lesser commemoration' $authority $true)
    }
}

# OCA needs one jurisdiction-level source for the complete service structures.
$ocaAuthority = @"
schema: typikon.authority/v0.1

id: oca-service-structures
title: Orthodox Church in America liturgical service texts and outlines
category: source
kind: authoritative
publisher: Orthodox Church in America
locator:
  scope: Service structures, fixed portions, and daily variable texts
reference:
  url: https://www.oca.org/liturgics/service-texts
  accessed: 2026-08-24
"@
Write-Utf8 (Join-Path $workspace 'typikon-oca\authorities\service-structures.yaml') $ocaAuthority

# Ordinary saint fixtures now own their actual contribution metadata.
$localContributions = @{
    'typikon-goarch\observances\saints\martyrs\paraskevi-rome.yaml' = 'goarch-dcs-2026-07-25-vespers'
    'typikon-goarch\observances\saints\martyrs\stephen-protomartyr-translation.yaml' = 'goarch-dcs-2026-08-01-vespers'
    'typikon-oca\observances\angels\archangels\archangel-michael-colossae.yaml' = 'oca-order-2026-09-06'
    'typikon-oca\observances\saints\monastics\pimen-great.yaml' = 'oca-order-2023-08-27'
}
foreach ($entry in $localContributions.GetEnumerator()) {
    $path = Join-Path $workspace $entry.Key
    $content = (Get-Content $path -Raw).TrimEnd()
    $content = [regex]::Replace($content, '(?ms)^services:\r?\n.*\z', '').TrimEnd()
    $title = ([regex]::Match($content, '(?m)^name: (.+)$')).Groups[1].Value
    $material = @"


services:
  great_vespers:
    lord_i_call:
      stichera:
        kind: hymn_set
        role: saint
        title: $title - stichera at Lord, I Call
        authority:
          - $($entry.Value)
"@
    if ($content -match '(?m)^  has_glory: true$') {
        $material += "`n" + @"
      glory:
        kind: sticheron
        role: doxastikon
        title: $title - doxastikon at Lord, I Call
        authority:
          - $($entry.Value)
"@
    }
    Write-Utf8 $path ($content + $material)
}

# The rule supplies cycle material and asks each observance for its own propers.
foreach ($packName in @('typikon-goarch','typikon-oca')) {
    $authorityBlock = if ($packName -eq 'typikon-oca') { "`nauthority:`n  - oca-ordinary-sunday-lord-i-call" } else { '' }
    $ordinaryRule = @"
schema: typikon.rule/v0.3

id: ordinary-sunday-six-stichera
when:
  service: great_vespers
  day:
    weekday: sunday
    phase: ordinary
  observance:
    rank: six-stichera
    properties:
      has_glory: true
emit:
  - section: lord_i_call
    component: stichera
    material:
      kind: hymn_set
      role: resurrectional
      title: Resurrectional stichera from the Octoechos
      attributes:
        tone: `$day.tone
    count: 6
  - section: lord_i_call
    component: stichera
    observance: true
    count: 4
  - section: lord_i_call
    component: glory
    observance: true
  - section: lord_i_call
    component: both_now
    material:
      kind: theotokion
      role: dogmatikon
      title: Appointed theotokion
      attributes:
        tone: `$day.tone$authorityBlock
"@
    Write-Utf8 (Join-Path $workspace "$packName\rules\ordinary-sunday-six-stichera.yaml") $ordinaryRule
}

$lesserRule = @"
schema: typikon.rule/v0.3

id: ordinary-sunday-lesser
when:
  service: great_vespers
  day:
    weekday: sunday
    phase: ordinary
  observance:
    rank: lesser
    properties:
      has_glory: true
emit:
  - section: lord_i_call
    component: stichera
    material:
      kind: hymn_set
      role: resurrectional
      title: Resurrectional stichera from the Octoechos
      attributes:
        tone: `$day.tone
    count: 7
  - section: lord_i_call
    component: stichera
    observance: true
    count: 3
  - section: lord_i_call
    component: glory
    observance: true
  - section: lord_i_call
    component: both_now
    material:
      kind: theotokion
      role: dogmatikon
      title: Appointed theotokion
      attributes:
        tone: `$day.tone
authority:
  - oca-ordinary-sunday-lord-i-call
"@
Write-Utf8 (Join-Path $workspace 'typikon-oca\rules\ordinary-sunday-lesser.yaml') $lesserRule
