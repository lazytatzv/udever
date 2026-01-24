use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Select};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command};
use std::thread;
use std::time::Duration;
use nix::unistd::getuid;

#[derive(Parser)]
#[command(name = "udever")]
struct Args {
    #[arg(short, long)]
    id: Option<String>,

    /// Generate shell completions
    #[arg(long, value_enum)]
    completion: Option<Shell>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args = Args::parse();
    
    if let Some(shell) = args.completion {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "udever", &mut io::stdout());
        return Ok(());
    }

    // if unsafe { libc::getuid() != 0 } {
    //     eprintln!("Error: Run as root.");
    //     std::process::exit(1);
    // }

    if getuid().as_raw() != 0 {
        eprintln!("Error: Run as root.");
        std::process::exit(1);
    }

    let theme = ColorfulTheme::default();

    if args.id.is_some() {
        create_new_rule(&theme, args.id)?;
    } else {
        loop {
            let options = &[
                "Create new rule",
                "Edit existing rule",
                "Delete rule",
                "Exit"
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
                _ => break,
            }
        }
    }
    Ok(())
}

fn create_new_rule(theme: &ColorfulTheme, arg_id: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // idVendor and ipProduct are required(hex)
    
    let (vendor, product, desc) = if let Some(id) = arg_id {
        let p: Vec<&str> = id.split(':').collect();
        if p.len() != 2 { return Err("Invalid ID".into()); }
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
        .default(false)
        .interact()? 
    {
        let default = format!("{}_{}", vendor, product);
        Some(Input::<String>::with_theme(theme).with_prompt("Symlink Name").default(default).interact_text()?)
    } else {
        None
    };

    let perms = &[
        "Current user only (uaccess)",
        "Everyone (mode 0666)",
        "Group 'uucp' (mode 0660)",
        "Open in editor...",
    ];
    let perm_idx = Select::with_theme(theme)
        .with_prompt("Permission")
        .default(0)
        .items(perms)
        
        .interact()?;

    let perm_rule = match perm_idx {
        1 => "MODE=\"0666\"".to_string(),
        2 => "GROUP=\"uucp\", MODE=\"0660\"".to_string(),
        3 => "EDITOR".to_string(),
        _ => "TAG+=\"uaccess\"".to_string(),
    };

    let name_base = symlink.clone().unwrap_or_else(|| format!("{}-{}", vendor, product));
    let filename = format!("/etc/udev/rules.d/99-{}.rules", name_base);
    
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
        println!("\n--- Preview: {} ---", filename);
        println!("{}", rule.trim());
        println!("-----------------------------------");
        
        if !Confirm::with_theme(theme)
            .with_prompt("Write to file?")
            .default(true)
            .interact()? {
                println!("Aborted.");
                return Ok(());
        }
    }

    fs::write(&filename, rule)?;
    println!("File created.");

    if perm_idx == 3 {
        open_editor(&filename)?;
    }

    apply_and_verify(&symlink)?;
    Ok(())
}

fn manage_rules(theme: &ColorfulTheme, action: &str) -> Result<(), Box<dyn std::error::Error>> {
    let paths = fs::read_dir("/etc/udev/rules.d/")?;
    let mut files: Vec<String> = paths.filter_map(|e| e.ok()).map(|e| e.path().to_string_lossy().into_owned()).filter(|s| s.ends_with(".rules")).collect();
    files.sort();
    files.push("Go Back".to_string());
    if files.len() == 1 { println!("No rules found."); return Ok(()); }

    let selection = FuzzySelect::with_theme(theme)
        .with_prompt(format!("Select rule to {} (Type to search)", action))
        .items(&files)
        .default(0)
        
        .interact()?;

    if selection == files.len() - 1 { return Ok(()); }
    let target = &files[selection];

    if action == "edit" { open_editor(target)?; apply_and_verify(&None)?; } 
    else if action == "delete" {
        if Confirm::with_theme(theme).with_prompt(format!("Delete {}?", target)).interact()? {
            fs::remove_file(target)?; println!("Deleted."); apply_and_verify(&None)?;
        }
    }
    Ok(())
}

fn open_editor(filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    Command::new(editor).arg(filepath).status()?;
    Ok(())
}

fn apply_and_verify(symlink: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Reloading udev rules...");
    Command::new("udevadm").arg("control").arg("--reload").status()?;
    Command::new("udevadm").args(&["trigger", "--action=add", "--subsystem-match=usb"]).status()?;
    if let Some(s) = symlink {
        let path = Path::new("/dev").join(s);
        print!("Waiting for device...");
        for _ in 0..20 {
            if path.exists() { println!("\nSuccess: {:?}", path); return Ok(()); }
            thread::sleep(Duration::from_millis(100));
            print!(".");
            std::io::stdout().flush()?;
        }
        eprintln!("\nWarning: Device not found yet.");
    } else { println!("Applied."); }
    Ok(())
}

// Returns (idVendor, idProduct, Description)
fn select_device(
    theme: &ColorfulTheme,
) -> Result<Option<(String, String, String)>, Box<dyn std::error::Error>> {

    // (vid, pid, name, bus)
    let mut items: Vec<(String, String, String, String)> = Vec::new();
    let sys_path = Path::new("/sys/bus/usb/devices");

    for entry in fs::read_dir(sys_path)? {
        let entry = entry?;
        let path = entry.path();

        let id_vendor = fs::read_to_string(path.join("idVendor")).ok();
        let id_product = fs::read_to_string(path.join("idProduct")).ok();

        if let (Some(id_vendor), Some(id_product)) = (id_vendor, id_product) {
            let product = fs::read_to_string(path.join("product")).unwrap_or_default();
            let manu = fs::read_to_string(path.join("manufacturer")).unwrap_or_default();

            let name = format!("{} {}", manu.trim(), product.trim())
                .trim()
                .to_string();

            let bus = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            items.push((
                id_vendor.trim().to_string(),
                id_product.trim().to_string(),
                name,
                format!("@{}", bus),
            ));
        }
    }

    if items.is_empty() {
        return Err("No USB devices found".into());
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

