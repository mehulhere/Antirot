import type { Metadata, Viewport } from "next";
import "./globals.css";

export const metadata: Metadata = {
    title: "Antirot Lab",
    description: "Backend and app-functionality test console for Antirot."
};

export const viewport: Viewport = {
    width: "device-width",
    initialScale: 1,
    themeColor: "#050507"
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
    return (
        <html lang="en">
            <body>{children}</body>
        </html>
    );
}
