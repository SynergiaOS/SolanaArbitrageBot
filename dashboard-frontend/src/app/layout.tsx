import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Sidebar } from "@/components/layout/Sidebar";
import { TopHeader } from "@/components/layout/TopHeader";
import { ThemeProvider } from "@/components/providers/theme-provider";
import { WebSocketProvider } from "@/components/providers/websocket-provider";
import { SolanaWalletProvider, EnhancedWalletProvider } from "@/components/providers/wallet-provider";
import { EnhancedWebSocketProvider } from "@/components/providers/enhanced-websocket-provider";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Enhanced Solana Arbitrage Bot Dashboard",
  description: "Real-time monitoring and control dashboard for Enhanced Solana Arbitrage Bot with Sniper, GEPA, and Kestra integration",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        <ThemeProvider
          attribute="class"
          defaultTheme="dark"
          enableSystem
          disableTransitionOnChange
        >
          <SolanaWalletProvider>
            <EnhancedWalletProvider>
              <WebSocketProvider>
                <EnhancedWebSocketProvider>
                  <div className="flex h-screen bg-background">
                    {/* Sidebar */}
                    <Sidebar />

                    {/* Main content area */}
                    <div className="flex-1 flex flex-col overflow-hidden">
                      <TopHeader />
                      <main className="flex-1 overflow-auto">
                        {children}
                      </main>
                    </div>
                  </div>
                </EnhancedWebSocketProvider>
              </WebSocketProvider>
            </EnhancedWalletProvider>
          </SolanaWalletProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
