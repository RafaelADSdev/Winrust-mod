// purge              show what would be removed (nothing is modified)
// purge --apply      REMOVE it, for good (admin). There is no undo: only a log of what went is kept
// purge --ask        list, then ask to type YES before removing (what the topbar menu starts, in its own window)
// purge --onedrive   (with --apply) also uninstall OneDrive      purge --edge   (with --apply) also try to uninstall Edge
//
// What Tiny11 cuts out of the image, done on a running system: bundled Store apps (installed and provisioned, so new
// users do not get them back) and the telemetry scheduled tasks. Never touched: the Store, Settings, the shell hosts,
// runtimes and everything in PROTECTED. Reinstalling an app means Store or winget; deleted tasks do not come back.
use std::{env, fs, io::Write, path::PathBuf, process::Command};

/// Apps to remove: a package name starts with one of these (a name of 5+ letters without a dot also matches anywhere).
const JUNK: &[&str] = &[
    "Clipchamp.Clipchamp", "Microsoft.BingNews", "Microsoft.BingWeather", "Microsoft.BingSearch", "Microsoft.Copilot",
    "Microsoft.Windows.Ai.Copilot.Provider", "Microsoft.549981C3F5F10", // Cortana
    "Microsoft.GamingApp", "Microsoft.XboxGameOverlay", "Microsoft.XboxGamingOverlay", "Microsoft.XboxSpeechToTextOverlay", "Microsoft.Xbox.TCUI",
    "Microsoft.GetHelp", "Microsoft.Getstarted", "Microsoft.MicrosoftOfficeHub", "Microsoft.Office.OneNote", "Microsoft.MicrosoftSolitaireCollection",
    "Microsoft.People", "Microsoft.PowerAutomateDesktop", "Microsoft.Todos", "Microsoft.WindowsAlarms", "microsoft.windowscommunicationsapps",
    "Microsoft.WindowsFeedbackHub", "Microsoft.WindowsMaps", "Microsoft.WindowsSoundRecorder", "Microsoft.YourPhone", "Microsoft.ZuneMusic",
    "Microsoft.ZuneVideo", "MicrosoftCorporationII.MicrosoftFamily", "MicrosoftCorporationII.QuickAssist", "MicrosoftTeams", "MSTeams",
    "Microsoft.OutlookForWindows", "Microsoft.Windows.DevHome", "MicrosoftWindows.Client.WebExperience", // Widgets
    "Microsoft.SkypeApp", "Microsoft.MixedReality.Portal", "Microsoft.Microsoft3DViewer", "Microsoft.Print3D", "Microsoft.Wallet",
    "SpotifyAB.SpotifyMusic", "Disney.", "Netflix", "TikTok", "Facebook", "Instagram", "LinkedIn", "Twitter", "AmazonVideo", "king.com.", "Duolingo",
];

/// Never removed, whatever JUNK says: removing these breaks Settings, Start, the Store, sign-in or other apps.
const PROTECTED: &[&str] = &[
    "Microsoft.WindowsStore", "Microsoft.StorePurchaseApp", "Microsoft.DesktopAppInstaller", "windows.immersivecontrolpanel",
    "Microsoft.Windows.ShellExperienceHost", "Microsoft.Windows.StartMenuExperienceHost", "MicrosoftWindows.Client.", "Microsoft.Windows.",
    "Microsoft.NET", "Microsoft.VCLibs", "Microsoft.UI.Xaml", "Microsoft.WindowsAppRuntime", "Microsoft.SecHealthUI", "Microsoft.AAD.BrokerPlugin",
    "Microsoft.AccountsControl", "Microsoft.LockApp", "Microsoft.XboxIdentityProvider", "Microsoft.GamingServices", "Microsoft.WindowsTerminal",
    "Microsoft.WindowsCalculator", "Microsoft.Paint", "Microsoft.ScreenSketch", "Microsoft.Windows.Photos", "Microsoft.WindowsNotepad",
];

/// Telemetry scheduled tasks (Tiny11 deletes the same ones).
const TASKS: &[&str] = &[
    r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser",
    r"\Microsoft\Windows\Application Experience\ProgramDataUpdater",
    r"\Microsoft\Windows\Application Experience\StartupAppTask",
    r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator",
    r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip",
    r"\Microsoft\Windows\Customer Experience Improvement Program\KernelCeipTask",
    r"\Microsoft\Windows\Feedback\Siuf\DmClient",
    r"\Microsoft\Windows\Feedback\Siuf\DmClientOnScenarioDownload",
    r"\Microsoft\Windows\Windows Error Reporting\QueueReporting",
];

fn starts(name: &str, list: &[&str]) -> bool {
    let n = name.to_lowercase();
    list.iter().any(|p| n.starts_with(&p.to_lowercase()))
}

/// Removable junk. "Microsoft.Windows." is protected as a whole, except the few items that are JUNK by their full name.
fn is_junk(name: &str) -> bool {
    let keep = starts(name, PROTECTED) && !starts(name, &["Microsoft.Windows.Ai.Copilot.Provider", "Microsoft.Windows.DevHome", "MicrosoftWindows.Client.WebExperience"]);
    let n = name.to_lowercase();
    !keep && JUNK.iter().any(|p| n.starts_with(&p.to_lowercase()) || (p.len() >= 5 && !p.contains('.') && n.contains(&p.to_lowercase())))
}

enum Item {
    Appx { full: String, name: String },
    Prov { pkg: String, name: String },
}

fn run(prog: &str, args: &[&str]) -> (bool, String) {
    match Command::new(prog).args(args).output() {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(e) => (false, e.to_string()),
    }
}

fn ps(script: &str) -> (bool, String) {
    run("powershell", &["-NoProfile", "-NonInteractive", "-Command", script])
}

/// `appx|FullName|Name|NonRemovable` and `prov|PackageName|DisplayName` lines of the listing script.
fn parse(out: &str) -> Vec<Item> {
    let mut v = vec![];
    for l in out.lines().map(str::trim) {
        let f: Vec<&str> = l.split('|').collect();
        // a name with a quote would break out of the removal command: such a package is left alone
        if f.iter().any(|x| x.contains('\'')) {
            continue;
        }
        match f.as_slice() {
            ["appx", full, name, nonremovable] if *nonremovable != "True" && is_junk(name) => v.push(Item::Appx { full: full.to_string(), name: name.to_string() }),
            ["prov", pkg, name] if is_junk(name) => v.push(Item::Prov { pkg: pkg.to_string(), name: name.to_string() }),
            _ => {}
        }
    }
    v
}

fn found() -> Result<Vec<Item>, String> {
    let (ok, out) = ps(r#"Get-AppxPackage -AllUsers | ForEach-Object { "appx|$($_.PackageFullName)|$($_.Name)|$($_.NonRemovable)" }; Get-AppxProvisionedPackage -Online | ForEach-Object { "prov|$($_.PackageName)|$($_.DisplayName)" }"#);
    if !ok && out.trim().is_empty() {
        return Err(winrust_common::pt!("cannot list apps (run as administrator)").into());
    }
    Ok(parse(&out))
}

fn task_exists(t: &str) -> bool {
    run("schtasks.exe", &["/Query", "/TN", t]).0
}

fn onedrive_setup() -> Option<PathBuf> {
    let root = PathBuf::from(env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()));
    ["SysWOW64", "System32"].iter().map(|d| root.join(d).join("OneDriveSetup.exe")).find(|p| p.is_file())
}

fn edge_setup() -> Option<PathBuf> {
    let base = PathBuf::from(env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into())).join(r"Microsoft\Edge\Application");
    let mut vs: Vec<PathBuf> = fs::read_dir(base).ok()?.flatten().map(|e| e.path().join(r"Installer\setup.exe")).filter(|p| p.is_file()).collect();
    vs.sort();
    vs.pop()
}

fn plan(od: bool, edge: bool) {
    println!("{}", winrust_common::pf!("Dry run: nothing is removed. --apply REMOVES it FOR GOOD (no undo).\n", "Simulação: nada foi removido. --apply exclui os itens permanentemente, sem opção de desfazer.\n"));
    match found() {
        Ok(items) => {
            for i in &items {
                match i {
                    Item::Appx { name, .. } => println!("{}", winrust_common::pf!("  app (installed)    {name}", "  aplicativo (instalado) {name}")),
                    Item::Prov { name, .. } => println!("{}", winrust_common::pf!("  app (provisioned)  {name}", "  aplicativo (provisionado) {name}")),
                }
            }
            println!("{}", winrust_common::pf!("  {} app entries found", "  Entradas do aplicativo {} encontradas", items.len()));
        }
        Err(e) => println!("  {e}"),
    }
    for t in TASKS.iter().filter(|t| task_exists(t)) {
        println!("{}", winrust_common::pf!("  task               {t}", "  tarefa {t}"));
    }
    if od {
        println!("{}", winrust_common::pf!("  OneDrive           {}", "  OneDrive {}", onedrive_setup().map_or(winrust_common::pt!("not present").into(), |p| p.display().to_string())));
    }
    if edge {
        println!("  Edge               {}", edge_setup().map_or(winrust_common::pt!("not present").into(), |p| p.display().to_string()));
    }
}

fn apply(od: bool, edge: bool) -> Result<(), String> {
    let dir = PathBuf::from(env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into())).join("winrust-purge");
    fs::create_dir_all(&dir).map_err(|e| winrust_common::pf!("{}: {e} (run as admin)", "{}: {e} (executar como administrador)", dir.display()))?;
    let mut log = fs::OpenOptions::new().create(true).append(true).open(dir.join("removed.txt")).map_err(|e| e.to_string())?;
    let mut failed = 0;
    let mut did = |what: String, ok: bool| {
        println!("{}  {what}", if ok { winrust_common::pt!("removed") } else { winrust_common::pt!("FAILED ") });
        if ok {
            let _ = writeln!(log, "{what}");
        } else {
            failed += 1;
        }
    };
    for i in found()? {
        match i {
            Item::Appx { full, name } => did(winrust_common::pf!("app {name}", "aplicativo {name}"), ps(&format!("Remove-AppxPackage -Package '{full}' -AllUsers -ErrorAction Stop")).0),
            Item::Prov { pkg, name } => did(winrust_common::pf!("provisioned {name}", "{name} provisionado"), ps(&format!("Remove-AppxProvisionedPackage -Online -PackageName '{pkg}' -ErrorAction Stop")).0),
        }
    }
    for t in TASKS.iter().filter(|t| task_exists(t)) {
        did(winrust_common::pf!("task {t}", "tarefa {t}"), run("schtasks.exe", &["/Delete", "/TN", t, "/F"]).0);
    }
    if od {
        let _ = run("taskkill", &["/f", "/im", "OneDrive.exe"]);
        match onedrive_setup() {
            Some(p) => did("OneDrive".into(), run(&p.display().to_string(), &["/uninstall"]).0),
            None => println!("{}", winrust_common::pf!("skip     OneDrive: not present", "pular OneDrive: não presente")),
        }
    }
    if edge {
        match edge_setup() {
            // Windows only lets Edge go outside the EU when this value exists
            Some(p) => {
                let _ = run("reg", &["add", r"HKLM\SOFTWARE\Microsoft\EdgeUpdateDev", "/v", "AllowUninstall", "/t", "REG_SZ", "/d", "", "/f"]);
                did(winrust_common::pt!("Edge (best effort, Windows may refuse)").into(), run(&p.display().to_string(), &["--uninstall", "--system-level", "--verbose-logging", "--force-uninstall"]).0)
            }
            None => println!("{}", winrust_common::pf!("skip     Edge: not present", "ignorado  Edge: não encontrado")),
        }
    }
    println!("{}", winrust_common::pf!("\nThis cannot be undone. A log of what was removed: {}", "\nIsto não pode ser desfeito. Um registro do que foi removido: {}", dir.join("removed.txt").display()));
    if failed > 0 { Err(winrust_common::pf!("{failed} item(s) failed (run as administrator)", "Os itens {failed} falharam (executar como administrador)")) } else { Ok(()) }
}

fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    let has = |f: &str| a.iter().any(|x| x == f);
    let r = if has("--ask") {
        plan(has("--onedrive"), has("--edge"));
        print!("{}", winrust_common::pf!("
This REMOVES the apps above FOR GOOD, there is no undo. Type YES to continue: ", "\nOs aplicativos acima serão excluídos permanentemente. Esta ação é irreversível. Digite YES para continuar: "));
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        let r = if line.trim() == "YES" { apply(has("--onedrive"), has("--edge")) } else { println!("{}", winrust_common::pf!("Nothing removed.", "Nada removido.")); Ok(()) };
        println!("{}", winrust_common::pf!("
Press Enter to close.", "\nPressione Enter para fechar."));
        let _ = std::io::stdin().read_line(&mut line);
        r
    } else if has("--apply") {
        apply(has("--onedrive"), has("--edge"))
    } else {
        plan(has("--onedrive"), has("--edge"));
        Ok(())
    };
    if let Err(e) = r {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junk_goes_and_the_system_stays() {
        for n in ["Microsoft.BingNews", "Clipchamp.Clipchamp", "Microsoft.ZuneMusic", "MicrosoftTeams", "king.com.CandyCrushSaga", "Microsoft.Windows.DevHome", "MicrosoftWindows.Client.WebExperience", "5319275A.Facebook"] {
            assert!(is_junk(n), "{n}");
        }
        for n in ["Microsoft.WindowsStore", "windows.immersivecontrolpanel", "Microsoft.Windows.ShellExperienceHost", "Microsoft.Windows.StartMenuExperienceHost", "Microsoft.WindowsCalculator", "Microsoft.XboxIdentityProvider", "MicrosoftWindows.Client.CBS", "Microsoft.VCLibs.140.00", "Microsoft.Windows.Photos"] {
            assert!(!is_junk(n), "{n}");
        }
    }

    #[test]
    fn parses_listing_and_skips_nonremovable_and_quotes() {
        let out = "appx|Microsoft.BingNews_1_x64__8wekyb|Microsoft.BingNews|False\nappx|Microsoft.ZuneMusic_1|Microsoft.ZuneMusic|True\nprov|Clipchamp.Clipchamp_1|Clipchamp.Clipchamp\nprov|X'Y|Microsoft.BingNews\nappx|a|Microsoft.WindowsStore|False\n";
        assert_eq!(parse(out).len(), 2);
    }
}
