import { readFileSync } from "node:fs";

const html = readFileSync("website/index.html", "utf8");
const css = readFileSync("website/style.css", "utf8");

const checks = [
    {
        label: "hero leads with app download copy",
        pass: html.includes("Download Antirot"),
    },
    {
        label: "Android APK download is present",
        pass: html.includes('href="https://github.com/mehulhere/Antirot/releases/latest/download/antirot.apk"'),
    },
    {
        label: "iOS IPA download is present",
        pass: html.includes('href="https://testflight.apple.com"'),
    },
    {
        label: "new phone mockup visual is present",
        pass: html.includes("phone-mockup"),
    },
    {
        label: "install notes section is present",
        pass: html.includes('id="install-notes"'),
    },
    {
        label: "old focus dial hero has been removed",
        pass: !html.includes("focus-dial-hero"),
    },
    {
        label: "native app background token is present",
        pass: css.includes("#0A0A0A"),
    },
    {
        label: "native app surface token is present",
        pass: css.includes("#141414"),
    },
    {
        label: "native app elevated token is present",
        pass: css.includes("#1C1C1E"),
    },
    {
        label: "native app accent token is present",
        pass: css.includes("#E63946"),
    },
];

const failures = checks.filter((check) => !check.pass);

if (failures.length > 0) {
    console.error("Website landing smoke test failed:");
    for (const failure of failures) {
        console.error(`- ${failure.label}`);
    }
    process.exit(1);
}

console.log("Website landing smoke test passed.");
