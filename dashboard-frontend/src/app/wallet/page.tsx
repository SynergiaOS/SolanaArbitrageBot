"use client";

import { WalletButton } from "@/components/wallet/WalletButton";
import { SniperTransactionManager } from "@/components/sniper/SniperTransactionManager";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Wallet, Target, History } from "lucide-react";

export default function WalletPage() {
  return (
    <div className="p-6 space-y-6">
      {/* Page Header */}
      <div className="flex items-center gap-3">
        <Wallet className="h-8 w-8 text-primary" />
        <div>
          <h1 className="text-3xl font-bold">Wallet & Transactions</h1>
          <p className="text-muted-foreground">
            Manage your wallet connection and approve sniper transactions
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Wallet Connection */}
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Wallet className="h-5 w-5" />
                Wallet Connection
              </CardTitle>
            </CardHeader>
            <CardContent>
              <WalletButton />
            </CardContent>
          </Card>

          {/* Instructions */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Target className="h-5 w-5" />
                How It Works
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-2">
                <h4 className="font-medium">1. Connect Your Wallet</h4>
                <p className="text-sm text-muted-foreground">
                  Connect your Solana wallet (Phantom, Solflare, etc.) to enable transaction signing.
                </p>
              </div>
              
              <div className="space-y-2">
                <h4 className="font-medium">2. Enable Auto-Snipe</h4>
                <p className="text-sm text-muted-foreground">
                  Turn on auto-sniping to automatically detect and prepare transactions for new tokens.
                </p>
              </div>
              
              <div className="space-y-2">
                <h4 className="font-medium">3. Approve Transactions</h4>
                <p className="text-sm text-muted-foreground">
                  Review and approve each snipe transaction manually for maximum security.
                </p>
              </div>
              
              <div className="space-y-2">
                <h4 className="font-medium">4. Monitor Results</h4>
                <p className="text-sm text-muted-foreground">
                  Track your transaction history and performance in real-time.
                </p>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Transaction Manager */}
        <div>
          <SniperTransactionManager />
        </div>
      </div>
    </div>
  );
}
