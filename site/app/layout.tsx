import { Inter } from "next/font/google";
import { Provider } from "@/components/provider";
import "./global.css";
import type { Metadata } from "next";

const inter = Inter({
  subsets: ["latin"],
});

export const metadata: Metadata = {
  metadataBase: new URL("https://mathematic-inc.github.io/earl"),
  title: "Earl",
  description: "AI-safe CLI for AI agents",
  openGraph: {
    title: "Earl",
    description: "AI-safe CLI for AI agents",
    url: "https://mathematic-inc.github.io/earl",
    siteName: "Earl",
    images: [
      {
        url: "/earl/social-preview.jpg",
        width: 1280,
        height: 720,
        alt: "Earl - AI-safe CLI for AI agents",
      },
    ],
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Earl",
    description: "AI-safe CLI for AI agents",
    images: ["/earl/social-preview.jpg"],
  },
};

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={inter.className} suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
