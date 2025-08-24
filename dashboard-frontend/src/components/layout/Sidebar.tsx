"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useWebSocket } from "@/components/providers/websocket-provider";
import {
  BarChart3,
  TrendingUp,
  Settings,
  Activity,
  DollarSign,
  AlertTriangle,
  History,
  Target,
  Zap,
  Brain,
  Workflow,
  Shield,
  PieChart,
  Sliders,
  Wallet,
  TestTube
} from "lucide-react";

const navigation = [
  { name: "Dashboard", href: "/", icon: BarChart3, category: "main" },
  { name: "Trading", href: "/trading", icon: TrendingUp, category: "main" },
  { name: "Positions", href: "/positions", icon: DollarSign, category: "main" },
  { name: "Opportunities", href: "/opportunities", icon: Target, category: "main" },

  // Enhanced Components
  { name: "Sniper Bot", href: "/sniper", icon: Zap, category: "enhanced" },
  { name: "Wallet & Transactions", href: "/wallet", icon: Wallet, category: "enhanced" },
  { name: "Web3 Test", href: "/test-web3", icon: TestTube, category: "enhanced" },
  { name: "GEPA Optimizer", href: "/gepa", icon: Brain, category: "enhanced" },
  { name: "Kestra Workflows", href: "/kestra", icon: Workflow, category: "enhanced" },

  // Analytics & Management
  { name: "Analytics", href: "/analytics", icon: Activity, category: "analytics" },
  { name: "Risk Management", href: "/risk", icon: Shield, category: "analytics" },
  { name: "Performance", href: "/performance", icon: PieChart, category: "analytics" },
  { name: "History", href: "/history", icon: History, category: "analytics" },

  // System
  { name: "Configuration", href: "/config", icon: Sliders, category: "system" },
  { name: "Alerts", href: "/alerts", icon: AlertTriangle, category: "system" },
  { name: "Settings", href: "/settings", icon: Settings, category: "system" },
];

const categories = [
  { name: "Main", key: "main" },
  { name: "Enhanced Components", key: "enhanced" },
  { name: "Analytics", key: "analytics" },
  { name: "System", key: "system" },
];

export function Sidebar() {
  const pathname = usePathname();
  const { isConnected, systemHealth } = useWebSocket();

  const getStatusColor = () => {
    if (!isConnected) return "bg-red-500";
    if (systemHealth?.botStatus === "online") return "bg-green-500";
    if (systemHealth?.botStatus === "error") return "bg-red-500";
    return "bg-yellow-500";
  };

  const getStatusText = () => {
    if (!isConnected) return "Disconnected";
    if (systemHealth?.botStatus === "online") return "Bot Active";
    if (systemHealth?.botStatus === "error") return "Bot Error";
    return "Bot Offline";
  };

  return (
    <div className="w-64 bg-card border-r border-border flex flex-col">
      {/* Logo */}
      <div className="p-6 border-b border-border">
        <h1 className="text-xl font-bold text-foreground">
          Enhanced Arbitrage Bot
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          Sniper • GEPA • Kestra
        </p>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-4 space-y-6 overflow-y-auto">
        {categories.map((category) => (
          <div key={category.key}>
            <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
              {category.name}
            </h3>
            <div className="space-y-1">
              {navigation
                .filter((item) => item.category === category.key)
                .map((item) => {
                  const isActive = pathname === item.href;
                  return (
                    <Link
                      key={item.name}
                      href={item.href}
                      className={`flex items-center px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                        isActive
                          ? "bg-primary text-primary-foreground"
                          : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                      }`}
                    >
                      <item.icon className="mr-3 h-4 w-4" />
                      {item.name}
                    </Link>
                  );
                })}
            </div>
          </div>
        ))}
      </nav>

      {/* Status */}
      <div className="p-4 border-t border-border">
        <div className="space-y-2">
          <div className="flex items-center space-x-2">
            <div className={`w-2 h-2 rounded-full ${getStatusColor()}`}></div>
            <span className="text-sm text-muted-foreground">
              {getStatusText()}
            </span>
          </div>

          {systemHealth && (
            <div className="text-xs text-muted-foreground space-y-1">
              <div>Balance: {systemHealth.walletBalance.toFixed(2)} SOL</div>
              <div>Uptime: {Math.floor(systemHealth.uptime / 3600)}h</div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
