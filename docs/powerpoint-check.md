# Checking behaviour against PowerPoint (optional)

Some DeckCraft behaviour is defined by what PowerPoint does: an animation's default duration, a
date format, where a line breaks, what a transition writes to the file. If you have PowerPoint on
Windows you can ask it, through COM, and put the answer in your PR. **This is optional.** No test,
CI job or review depends on it, and a PR without it is fine.

## Rules (from [AGENTS.md](../AGENTS.md) §1.1 and §2)

- PowerPoint is **observed black-box**, on decks you build yourself. Never open or copy anything
  from inside the Office install.
- **Never commit anything PowerPoint produced:** no `.pptx` it saved, no screenshots or exported
  images, no XML pasted wholesale. Keep them in a temporary folder outside the checkout.
- What goes in a PR or a test is a **fact** you read (a number, an attribute value, "PowerPoint
  clamps this to 1 pt") and a test built on a deck DeckCraft generates. Don't copy Microsoft wording
  beyond feature and menu names.

## What you need

Windows with desktop PowerPoint (any recent Microsoft 365 or 2019+ build) and Windows PowerShell 5.
From WSL, run `powershell.exe -NoProfile -ExecutionPolicy Bypass ...` (set `WSL_INTEROP` if your
shell doesn't have it). macOS and Linux have no COM, so skip this there.

## The guard script

Save this as `pp-check.ps1` **outside the repository**. It runs a script body inside a PowerPoint
session and:

- refuses to start if PowerPoint is already open, so it never touches your own work;
- turns off alerts, and closes every presentation **without saving**;
- quits and kills only the PowerPoint process it started.

```powershell
param([Parameter(Mandatory)][string]$Body)
$ErrorActionPreference = 'Stop'
$W = Join-Path $env:TEMP 'dc-ppt-check'
New-Item -ItemType Directory -Force $W | Out-Null
if (Get-Process POWERPNT -ErrorAction SilentlyContinue) { throw 'PowerPoint is already running; not touching it' }
$ppt = New-Object -ComObject PowerPoint.Application
$mine = @(Get-Process POWERPNT -ErrorAction SilentlyContinue | ForEach-Object Id)
try {
    $ppt.DisplayAlerts = 1   # ppAlertsNone
    . $Body                  # sees $ppt and $W
} finally {
    try { while ($ppt.Presentations.Count -gt 0) { $p = $ppt.Presentations.Item(1); $p.Saved = -1; $p.Close() } } catch {}
    try { $ppt.Quit() } catch {}
    Start-Sleep -Seconds 1
    $mine | ForEach-Object { Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }
}
```

Run it with a body script. From WSL, wrap the call in `timeout 60` so a hung dialog can't wait
forever:

```powershell
.\pp-check.ps1 -Body .\body.ps1
```

## A body

Always build **new** presentations (`Presentations.Add`) and write only under `$W`. To compare file
output, have DeckCraft write a deck (`deckcraft-cli`, or `slide.new` and friends through the
[control protocol](control-protocol.md)), open it in PowerPoint **read-only**, read the property
you care about, and for file-level questions `SaveAs` a copy under `$W` and read its XML.

```powershell
# What does PowerPoint give a new Fade entrance as its default duration?
$p = $ppt.Presentations.Add(0)
$s = $p.Slides.Add(1, 12)                       # ppLayoutBlank
$box = $s.Shapes.AddShape(1, 50, 50, 200, 100)
$e = $s.TimeLine.MainSequence.AddEffect($box, 10)   # msoAnimEffectFade
"duration: $($e.Timing.Duration) s"
```

Print the PowerPoint version and build (`$ppt.Version`, `$ppt.Build`) with your result, because
defaults change between releases.

## Putting the result in a PR

Say what you ran and what you read, for example "PowerPoint 16.0 build 20527 through COM, new
presentation, `AddEffect` with `msoAnimEffectFade`, `Timing.Duration` = 0.5". Put the number in the
PR text and a test built on a DeckCraft-generated deck; leave the PowerPoint file out. Say plainly
when you did not run PowerPoint.

## Troubleshooting

- *"PowerPoint is already running"*: close it, or set `$ppt` from your own running instance by hand
  (and then don't use the cleanup block, which would quit it).
- *The call hangs*: a modal dialog is open in a window you can't see. Stop the `POWERPNT` process
  the script started (its id is in `$mine`) and run again.
- *`New-Object` fails with a class-not-registered error*: desktop PowerPoint isn't installed, or only
  the store or web edition is.
