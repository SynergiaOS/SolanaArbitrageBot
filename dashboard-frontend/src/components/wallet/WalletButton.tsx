"use client";

import React from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { useWalletModal } from '@solana/wallet-adapter-react-ui';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { 
  Wallet, 
  Copy, 
  ExternalLink, 
  RefreshCw,
  AlertCircle,
  CheckCircle,
  Loader2
} from 'lucide-react';
import { useEnhancedWallet } from '@/components/providers/wallet-provider';

export function WalletButton() {
  const { publicKey, wallet, connect, disconnect, connecting } = useWallet();
  const { setVisible } = useWalletModal();
  const { balance, balanceLoading, error, refreshBalance } = useEnhancedWallet();

  const handleConnect = React.useCallback(() => {
    if (!wallet) {
      setVisible(true);
    } else {
      connect().catch(() => {
        // Handle connection error
      });
    }
  }, [wallet, connect, setVisible]);

  const handleDisconnect = React.useCallback(() => {
    disconnect().catch(() => {
      // Handle disconnection error
    });
  }, [disconnect]);

  const copyAddress = React.useCallback(() => {
    if (publicKey) {
      navigator.clipboard.writeText(publicKey.toString());
      // TODO: Add toast notification
    }
  }, [publicKey]);

  const openInExplorer = React.useCallback(() => {
    if (publicKey) {
      window.open(`https://solscan.io/account/${publicKey.toString()}`, '_blank');
    }
  }, [publicKey]);

  if (!publicKey) {
    return (
      <Button 
        onClick={handleConnect}
        disabled={connecting}
        className="flex items-center gap-2"
      >
        {connecting ? (
          <Loader2 className="h-4 w-4 animate-spin" />
        ) : (
          <Wallet className="h-4 w-4" />
        )}
        {connecting ? 'Connecting...' : 'Connect Wallet'}
      </Button>
    );
  }

  return (
    <Card className="w-full max-w-md">
      <CardContent className="p-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <CheckCircle className="h-5 w-5 text-green-500" />
            <span className="font-medium">Wallet Connected</span>
          </div>
          <Badge variant="secondary" className="text-xs">
            {wallet?.adapter.name}
          </Badge>
        </div>

        {/* Wallet Address */}
        <div className="mb-3">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Address:</span>
            <div className="flex items-center gap-1">
              <button
                onClick={copyAddress}
                className="text-xs font-mono hover:text-primary transition-colors"
                title="Copy address"
              >
                {publicKey.toString().slice(0, 4)}...{publicKey.toString().slice(-4)}
              </button>
              <button onClick={copyAddress} className="p-1 hover:bg-muted rounded">
                <Copy className="h-3 w-3" />
              </button>
              <button onClick={openInExplorer} className="p-1 hover:bg-muted rounded">
                <ExternalLink className="h-3 w-3" />
              </button>
            </div>
          </div>
        </div>

        {/* Balance */}
        <div className="mb-3">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Balance:</span>
            <div className="flex items-center gap-2">
              {balanceLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : error ? (
                <div className="flex items-center gap-1 text-red-500">
                  <AlertCircle className="h-4 w-4" />
                  <span className="text-xs">Error</span>
                </div>
              ) : (
                <span className="font-medium">
                  {balance !== null ? `${balance.toFixed(4)} SOL` : '--'}
                </span>
              )}
              <button
                onClick={refreshBalance}
                disabled={balanceLoading}
                className="p-1 hover:bg-muted rounded"
                title="Refresh balance"
              >
                <RefreshCw className={`h-3 w-3 ${balanceLoading ? 'animate-spin' : ''}`} />
              </button>
            </div>
          </div>
        </div>

        {/* Error Display */}
        {error && (
          <div className="mb-3 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">
            {error}
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleDisconnect}
            className="flex-1"
          >
            Disconnect
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setVisible(true)}
            className="flex-1"
          >
            Change Wallet
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export function WalletStatus() {
  const { publicKey, connected } = useWallet();
  const { balance, error } = useEnhancedWallet();

  if (!connected || !publicKey) {
    return (
      <Badge variant="secondary" className="flex items-center gap-1">
        <AlertCircle className="h-3 w-3" />
        Not Connected
      </Badge>
    );
  }

  return (
    <Badge variant="default" className="flex items-center gap-1">
      <CheckCircle className="h-3 w-3" />
      {balance !== null ? `${balance.toFixed(2)} SOL` : 'Connected'}
    </Badge>
  );
}
