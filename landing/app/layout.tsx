import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Whop for Mac",
  description:
    "A native macOS app for whop.com. An unofficial, personal-use wrapper. Not affiliated with Whop.",
  openGraph: {
    title: "Whop for Mac",
    description: "A native macOS app for whop.com. Unofficial, personal use.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`${inter.variable} h-full antialiased`}>
      <body className="min-h-full">{children}</body>
    </html>
  );
}
