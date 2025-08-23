"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { 
  TrendingUp, 
  Briefcase, 
  Wallet, 
  Users, 
  MoreHorizontal,
  BarChart3,
  Settings,
  History
} from "lucide-react";

const navigation = [
  { name: "Trade", href: "/charts", icon: TrendingUp },
  { name: "Portfolio", href: "/portfolio", icon: Briefcase },
  { name: "Wallet", href: "/wallet", icon: Wallet },
  { name: "Control", href: "/control", icon: Settings },
  { name: "Config", href: "/config", icon: BarChart3 },
  { name: "History", href: "/history", icon: History },
  { name: "Affiliate", href: "/affiliate", icon: Users },
  { name: "More", href: "/more", icon: MoreHorizontal },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <div className="flex h-full w-64 flex-col bg-[var(--panel)] border-r border-[var(--border)]">
      {/* Logo */}
      <div className="flex h-16 items-center px-6 border-b border-[var(--border)]">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-[var(--primary)] flex items-center justify-center">
            <TrendingUp className="w-5 h-5 text-black" />
          </div>
          <span className="text-xl font-semibold text-white">SolanaBot</span>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-4 py-6 space-y-1">
        {navigation.map((item) => {
          const isActive = pathname === item.href;
          return (
            <Link
              key={item.name}
              href={item.href}
              className={`
                flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors
                ${isActive 
                  ? 'bg-[var(--primary)]/20 text-[var(--primary)] border border-[var(--primary)]/30' 
                  : 'text-gray-300 hover:text-white hover:bg-white/5'
                }
              `}
            >
              <item.icon className="w-5 h-5" />
              {item.name}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-[var(--border)]">
        <div className="text-xs text-gray-500">
          <div>Solana Arbitrage Bot</div>
          <div>v1.0.0</div>
        </div>
      </div>
    </div>
  );
}
