use argh::FromArgs;
use builder::ImageBuilder;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(FromArgs)]
#[argh(description = "vmxOS bootloader and kernel builder")]
struct Args {
    #[argh(switch, short = 'b')]
    #[argh(description = "boot the constructed image")]
    boot: bool,

    #[argh(switch, short = 'k')]
    #[argh(description = "use KVM acceleration")]
    kvm: bool,

    #[argh(switch, short = 'h')]
    #[argh(description = "use HAXM acceleration")]
    haxm: bool,

    #[argh(option, short = 'c')]
    #[argh(default = "4")]
    #[argh(description = "number of CPU cores")]
    cores: usize,

    #[argh(switch, short = 's')]
    #[argh(description = "redirect serial to stdio")]
    serial: bool,
}

fn main() {
    let img_path = build_img();
    let args: Args = argh::from_env();

    if args.boot {
        let mut cmd = Command::new("qemu-system-x86_64");

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let assets_dir = manifest_dir.join("assets");
        // let ext2_path = manifest_dir.join("ext2.img");

        let ovmf_path = assets_dir.join("OVMF_CODE.fd");
        let ovmf_config = format!("if=pflash,format=raw,file={}", ovmf_path.display());

        cmd.arg("-machine").arg("q35");
        cmd.arg("-drive").arg(ovmf_config);
        cmd.arg("-m").arg("8G");
        cmd.arg("-smp").arg(format!("cores={}", args.cores));
        cmd.arg("-cpu").arg("max,+x2apic");

        cmd.arg("-device").arg("ahci,id=ahci");
        let drive_config = format!(
            "format=raw,id=boot_disk,file={},if=none",
            img_path.display()
        );
        // cmd.arg("-device").arg("ide-hd,drive=boot_disk,bus=ahci.0");
        cmd.arg("-device").arg("nvme,drive=boot_disk,serial=1234");
        cmd.arg("-drive").arg(drive_config);

        // let ext2_config = format!(
        //     "if=none,format=raw,file={},id=ext2_disk",
        //     &ext2_path.display()
        // );
        // cmd.arg("-device").arg("ide-hd,drive=ext2_disk,bus=ahci.1");
        // cmd.arg("-device").arg("nvme,drive=ext2_disk,serial=1235");
        cmd.arg("-usb");
        cmd.arg("-device").arg("nec-usb-xhci,id=xhci");
        cmd.arg("-net").arg("nic");

        if args.kvm {
            cmd.arg("--enable-kvm");
        }
        if args.haxm {
            cmd.arg("-accel").arg("hax");
        }
        if args.serial {
            cmd.arg("-serial").arg("stdio");
        }

        let mut child = cmd.spawn().unwrap();
        child.wait().unwrap();
    }
}

fn build_img() -> PathBuf {
    let kernel_path = Path::new(env!("CARGO_BIN_FILE_KERNEL_kernel"));
    println!("Building UEFI disk image for kernel at {:#?}", &kernel_path);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assets_dir = manifest_dir.join("assets");
    let img_path = manifest_dir.parent().unwrap().join("vmxOS.img");

    let limine_elf = assets_dir.join("BOOTX64.EFI");
    let limine_config = assets_dir.join("limine.conf");

    ImageBuilder::build(
        kernel_path.to_path_buf(),
        limine_elf,
        limine_config,
        &img_path,
    )
    .expect("Failed to build UEFI disk image");
    println!("Created bootable UEFI disk image at {:#?}", &img_path);

    img_path
}
