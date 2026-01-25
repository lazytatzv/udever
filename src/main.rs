use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use dialoguer::{Confirm, FuzzySelect, Input, Select, theme::ColorfulTheme};
use nix::unistd::getuid;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use os_release::OsRelease;

#[derive(Parser)]
#[command(name = "udever")]
struct Args {
    #[arg(short, long)]
    id: Option<String>,

    /// Generate shell completions
    #[arg(long, value_enum)]
    completion: Option<Shell>,
}

fn main() -> Result<()> {

    let theme = ColorfulTheme::default();

    // UID0 is root
    if getuid().as_raw() != 0 {
        eprintln!("Error: Run as root.");
        std::process::exit(1);
    }

    udev_healthcheck(&theme)?;

    let args = Args::parse();

    if let Some(shell) = args.completion {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "udever", &mut io::stdout());
        return Ok(());
    }


    if args.id.is_some() {
        create_new_rule(&theme, args.id)?;
    } else {
        // Without any arguments (by default)
        loop {
            let options = &[
                "Create new rule",
                "Edit existing rule",
                "Delete rule",
                "Force Reload & Trigger",
                "Exit",
            ];

            let selection = Select::with_theme(&theme)
                .with_prompt("udever")
                .default(0)
                .items(options)
                .interact()?;

            match selection {
                0 => create_new_rule(&theme, None)?,
                1 => manage_rules(&theme, "edit")?,
                2 => manage_rules(&theme, "delete")?,
                3 => reload_udev(&theme)?,
                _ => break,
            }
        }
    }
    Ok(())
}

/*
// Experimental
fn view_udev_logs() -> Result<()> {
    println!("\n--- Recent udev logs ---");
    let output = Command::new("journalctl")
        .arg("-u")
        .arg("systemd-udevd")
        .arg("-n")
        .arg("10")
        .arg("--no-pager")
        .output()
        .context("Failed to read journalctl")?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("Could not retrieve logs.");
    }
    Ok(())
}
*/

// With systemd
fn udev_healthcheck(theme: &ColorfulTheme) -> Result<()> {
    let is_active = Command::new("systemctl")
        .arg("is-active")
        .arg("systemd-udevd")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if is_active {
        return Ok(());
    } 

    println!("udev daemon is NOT active.");

    if Confirm::with_theme(theme)
        .with_prompt("Should I try to start systemd-udevd for you?")
        .default(true)
        .interact()?
    {
        println!("Starting systemd-udevd...");
        let status = Command::new("systemctl")
            .arg("start")
            .arg("systemd-udevd")
            .status()
            .context("Failed to execute systemctl start")?;

        if status.success() {
            println!("Successfully started udev daemon.");
            Ok(())
        } else {
            anyhow::bail!("Failed to start udev. Please check 'systemctl status systemd-udevd.'");
            
        }
        
    } else {
        anyhow::bail!("Aborted. udev must be running to use this tool.");
    }
}

fn check_os() -> Result<String> {
    let os = OsRelease::new()?;
    println!("OS Name: {}", os.name);
    println!("OS ID: {}", os.id);

    Ok(os.id)
} 

// Use anyhow
fn reload_udev(theme: &ColorfulTheme) -> Result<()> {
    println!("Reloading udev rules...");

    if Confirm::with_theme(theme)
        .with_prompt("Reload udev?")
        .default(true)
        .interact()?
    {
        let status = Command::new("udevadm")
            .arg("control")
            .arg("--reload")
            .status()
            .context("Udev control failed to run")?;

        if status.success() {
            println!("udev rules reloaded.");
        } else {
            anyhow::bail!("udevadm control failed: {}", status);
        }

        let status = Command::new("udevadm")
            .arg("trigger")
            .arg("--action=add")
            .arg("--subsystem-match=usb")
            .status()
            .context("udevadm trigger failed")?;

        if status.success() {
            println!("udev triggerd");
        } else {
            anyhow::bail!("udev trigger failed {}", status);
        }
    }

    Ok(())
}

fn create_new_rule(theme: &ColorfulTheme, arg_id: Option<String>) -> Result<()> {
    // idVendor and ipProduct are required(hex)

    let (vendor, product, desc) = if let Some(id) = arg_id {
        let p: Vec<&str> = id.split(':').collect();
        if p.len() != 2 {
            anyhow::bail!("Invalid ID");
        }
        (p[0].to_string(), p[1].to_string(), "Target".to_string())
    } else {
        match select_device(theme)? {
            Some(data) => data,
            None => return Ok(()),
        }
    };

    println!("Target: {} [{}:{}]", desc, vendor, product);

    let symlink = if Confirm::with_theme(theme)
        .with_prompt("Create symlink?")
        .default(true) // You should create symlink
        .interact()?
    {
        let default = format!("{}_{}", vendor, product);
        Some(
            Input::<String>::with_theme(theme)
                .with_prompt("Symlink Name")
                .default(default)
                .interact_text()?,
        )
    } else {
        None
    };


    // memo: Should i use ID_LIKE instead of ID..??
    //
    let group_label = match check_os()?.as_str() {
        "arch"|"manjaro" => "Group 'uucp' (mode 0660)",
        "ubuntu"|"linuxmint"|"debian"|"fedora"|"rhel" => "Group 'dialout' (mode 0660)",
        _ => "Group 'dialout' (mode 0660) [OS type not detected]",
    };

    // Permissions
    let perms = vec![
        "Current user only (uaccess)",
        "Everyone (mode 0666)", // Not recommended
        group_label, // dynamic label
        "Open in editor...",
    ];

    let perm_idx = Select::with_theme(theme)
        .with_prompt("Permission")
        .default(0)
        .items(&perms)
        .interact()?;

    let perm_rule = match perm_idx {
        1 => "MODE=\"0666\"".to_string(),
        2 => "GROUP=\"uucp\", MODE=\"0660\"".to_string(),
        3 => "EDITOR".to_string(),
        _ => "TAG+=\"uaccess\"".to_string(),
    };

    let name_base = symlink
        .clone()
        .unwrap_or_else(|| format!("{}-{}", vendor, product));
    

    let filename = Path::new("/etc/udev/rules.d")
        .join(format!("99-{}.rules", name_base));

    let mut rule = if perm_rule == "EDITOR" {
        format!(
            "SUBSYSTEM==\"usb\", ACTION==\"add\", ATTRS{{idVendor}}==\"{}\", ATTRS{{idProduct}}==\"{}\", TAG+=\"uaccess\"\n",
            vendor, product
        )
    } else {
        format!(
            "SUBSYSTEM==\"usb\", ACTION==\"add\", ATTRS{{idVendor}}==\"{}\", ATTRS{{idProduct}}==\"{}\", {}",
            vendor, product, perm_rule
        )
    };

    if perm_rule != "EDITOR" {
        if let Some(ref s) = symlink {
            rule.push_str(&format!(", SYMLINK+=\"{}\"", s));
        }
        rule.push('\n');
    } else if let Some(ref s) = symlink {
        rule = rule.trim().to_string();
        rule.push_str(&format!(", SYMLINK+=\"{}\"\n", s));
    }

    if perm_rule != "EDITOR" {
        println!("\n--- Preview: {} ---", filename.display());
        println!("{}", rule.trim());
        println!("-----------------------------------");

        if !Confirm::with_theme(theme)
            .with_prompt("Write to file?")
            .default(true)
            .interact()?
        {
            println!("Aborted.");
            return Ok(());
        }
    }

    fs::write(&filename, rule)?;
    println!("File created.");

    if perm_idx == 3 {
        open_editor(&filename.to_string_lossy())?;
    }

    apply_and_verify(&symlink)?;
    Ok(())
}

fn manage_rules(theme: &ColorfulTheme, action: &str) -> Result<()> {
    let path = Path::new("/etc/udev/rules.d/");
    let entries = fs::read_dir(path)?;

    let mut files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().into_owned())
        .filter(|s| s.ends_with(".rules"))
        .collect();

    files.sort();
    files.push("Go Back".to_string());

    if files.len() == 1 {
        println!("No rules found.");
        return Ok(());
    }

    let selection = FuzzySelect::with_theme(theme)
        .with_prompt(format!("Select rule to {} (Type to search)", action))
        .items(&files)
        .default(0)
        .interact()?;

    if selection == files.len() - 1 {
        return Ok(());
    }
    let target = &files[selection];

    if action == "edit" {
        open_editor(target)?;
        apply_and_verify(&None)?;
    } else if action == "delete" {
        if Confirm::with_theme(theme)
            .with_prompt(format!("Delete {}?", target))
            .interact()?
        {
            fs::remove_file(target)?;
            println!("Deleted.");
            apply_and_verify(&None)?;
        }
    }
    Ok(())
}

// Should i explore PATH instead of the way below?
fn has_command(cmd: &str) -> bool {
    // Run via shell
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", cmd))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)

}

fn get_editor() -> Result<String> {
    match env::var("VISUAL").or_else(|_| env::var("EDITOR")) {
        Ok(v) => Ok(v),
        Err(_) => {
            eprintln!("Environment variables $VISUAL or $EDITOR are not set.");
            
            if has_command("nano") {
                println!("Nano found. Using it as default.");
                Ok("nano".to_string())
            } else if has_command("nvim") {
                println!("Nvim found. Using it as default.");
                Ok("nvim".to_string())
            } else if has_command("vim") {
                println!("Vim found. Using it as default.");
                Ok("vim".to_string())
            } else if has_command("vi") {
                println!("Vi found. Using it as default.");
                Ok("vi".to_string())
            } else {
                anyhow::bail!(
                    "No valid editor found (nano/vim/vi). \n\
                    Please set $EDITOR manually (e.g., export EDITOR=nvim)."
                )
            }
        }
    }
}

fn open_editor(filepath: &str) -> Result<()> {
    // I set nano as default cause it's possibly easy to use for even beginners
    let editor = get_editor()?;
    //println!("Your Editor is {}", editor);

    let status = Command::new(&editor)
        .arg(filepath)
        .status()
        .with_context(|| {
            format!(
                "Failed to launch editor '{}'. Ensure $EDITOR exists.",
                editor
            )
        })?;

    if !status.success() {
        anyhow::bail!("Editor terminated in a wrong way");
    }

    Ok(())
}

fn apply_and_verify(symlink: &Option<String>) -> Result<()> {
    println!("Reloading udev rules...");
    Command::new("udevadm")
        .arg("control")
        .arg("--reload")
        .status()?;
    Command::new("udevadm")
        .args(&["trigger", "--action=add", "--subsystem-match=usb"])
        .status()?;
    if let Some(s) = symlink {
        let path = Path::new("/dev").join(s);
        print!("Waiting for device...");
        for _ in 0..20 {
            if path.exists() {
                println!("\nSuccess: {:?}", path);
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
            print!(".");
            std::io::stdout().flush()?;
        }
        eprintln!("\nWarning: Device not found yet.");
    } else {
        println!("Applied.");
    }
    Ok(())
}

// Returns (idVendor, idProduct, Description)
fn select_device(theme: &ColorfulTheme) -> Result<Option<(String, String, String)>> {
    // (vid, pid, name, bus)
    let mut items: Vec<(String, String, String, String)> = Vec::new();
    let sys_path = Path::new("/sys/bus/usb/devices");

    for entry in fs::read_dir(sys_path)? {
        let entry = entry?;
        let path = entry.path();

        let id_vendor = fs::read_to_string(path.join("idVendor")).ok();
        let id_product = fs::read_to_string(path.join("idProduct")).ok();

        if let (Some(id_vendor), Some(id_product)) = (id_vendor, id_product) {
            // "1d6b" is Linux Foundation (Root Hub)
            // It is usually not configured
            if id_vendor.trim() == "1d6b" {
                continue;
            }


            let product = fs::read_to_string(path.join("product")).unwrap_or_default();
            let manu = fs::read_to_string(path.join("manufacturer")).unwrap_or_default();

            let name = format!("{} {}", manu.trim(), product.trim())
                .trim()
                .to_string();

            let bus = path.file_name().unwrap().to_string_lossy().to_string();

            items.push((
                id_vendor.trim().to_string(),
                id_product.trim().to_string(),
                name,
                format!("@{}", bus),
            ));
        }
    }

    if items.is_empty() {
        //return Err(anyhow::anyhow!("No USB devices found"));
        anyhow::bail!("No USB devices found");
    }

    // Sort by human-readable name
    items.sort_by(|a, b| a.2.cmp(&b.2));

    let name_w = items.iter().map(|x| x.2.len()).max().unwrap_or(0);

    // UI labels only
    let mut labels: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, (vid, pid, name, bus))| {
            format!(
                "{:>2}. {:<name_w$} [{:}:{:}] {}",
                i + 1,
                name,
                vid,
                pid,
                bus,
                name_w = name_w,
            )
        })
        .collect();

    labels.push(" Go Back".into());

    let idx = FuzzySelect::with_theme(theme)
        .with_prompt("Select USB Device (Type to search)")
        .default(0)
        .items(&labels)
        .interact()?;

    if idx == labels.len() - 1 {
        return Ok(None);
    }

    let (vid, pid, name, _) = &items[idx];
    Ok(Some((vid.clone(), pid.clone(), name.clone())))
}
