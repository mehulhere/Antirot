import type { Metadata, Viewport } from "next";
import { Space_Grotesk, DM_Sans, IBM_Plex_Mono } from "next/font/google";
import "./globals.css";

const spaceGrotesk = Space_Grotesk({
    subsets: ["latin"],
    weight: ["700"],
    variable: "--font-heading",
    display: "swap",
});

const dmSans = DM_Sans({
    subsets: ["latin"],
    weight: ["400", "500", "600"],
    variable: "--font-body",
    display: "swap",
});

const ibmPlexMono = IBM_Plex_Mono({
    subsets: ["latin"],
    weight: ["400", "500", "600"],
    variable: "--font-mono",
    display: "swap",
});

export const metadata: Metadata = {
    title: "Antirot Lab",
    description: "Backend and app-functionality test console for Antirot."
};

export const viewport: Viewport = {
    width: "device-width",
    initialScale: 1,
    themeColor: "#08070b"
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
    return (
        <html lang="en">
            <body className={`${spaceGrotesk.variable} ${dmSans.variable} ${ibmPlexMono.variable}`}>
                {children}
            </body>
        </html>
    );
}
