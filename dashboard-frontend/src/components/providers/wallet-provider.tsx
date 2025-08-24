"use client";

import React, { createContext, useContext, useMemo, ReactNode } from 'react';
import { WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { ConnectionProvider, WalletProvider, useWallet, useConnection } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  LedgerWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { clusterApiUrl } from '@solana/web3.js';

// Import wallet adapter CSS
import '@solana/wallet-adapter-react-ui/styles.css';

interface SolanaWalletProviderProps {
  children: ReactNode;
  network?: WalletAdapterNetwork;
  endpoint?: string;
}

export function SolanaWalletProvider({ 
  children, 
  network = WalletAdapterNetwork.Mainnet,
  endpoint 
}: SolanaWalletProviderProps) {
  // Get RPC endpoint from environment or use default
  const rpcEndpoint = useMemo(() => {
    if (endpoint) return endpoint;
    
    // Try to get from environment
    const envEndpoint = process.env.NEXT_PUBLIC_SOLANA_RPC_URL;
    if (envEndpoint) return envEndpoint;
    
    // Fallback to public endpoints
    if (network === WalletAdapterNetwork.Mainnet) {
      return process.env.NEXT_PUBLIC_SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com';
    }
    
    return clusterApiUrl(network);
  }, [endpoint, network]);

  // Configure supported wallets
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter({ network }),
      new LedgerWalletAdapter(),
    ],
    [network]
  );

  return (
    <ConnectionProvider endpoint={rpcEndpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          {children}
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

// Enhanced wallet context with additional utilities
interface EnhancedWalletContextType {
  isConnected: boolean;
  isConnecting: boolean;
  balance: number | null;
  balanceLoading: boolean;
  error: string | null;
  refreshBalance: () => Promise<void>;
}

const EnhancedWalletContext = createContext<EnhancedWalletContextType>({
  isConnected: false,
  isConnecting: false,
  balance: null,
  balanceLoading: false,
  error: null,
  refreshBalance: async () => {},
});

export function useEnhancedWallet() {
  return useContext(EnhancedWalletContext);
}

interface EnhancedWalletProviderProps {
  children: ReactNode;
}

export function EnhancedWalletProvider({ children }: EnhancedWalletProviderProps) {
  const [balance, setBalance] = React.useState<number | null>(null);
  const [balanceLoading, setBalanceLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // Import wallet hooks - these need to be inside the provider tree
  const { publicKey, connected, connecting } = useWallet();
  const { connection } = useConnection();

  const isConnected = connected && !!publicKey;
  const isConnecting = connecting;

  const refreshBalance = React.useCallback(async () => {
    if (!isConnected || !publicKey) return;

    setBalanceLoading(true);
    setError(null);

    try {
      const balance = await connection.getBalance(publicKey);
      setBalance(balance / 1000000000); // Convert lamports to SOL
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch balance');
    } finally {
      setBalanceLoading(false);
    }
  }, [isConnected, publicKey, connection]);

  // Auto-refresh balance when connected
  React.useEffect(() => {
    if (isConnected) {
      refreshBalance();
      
      // Refresh every 30 seconds
      const interval = setInterval(refreshBalance, 30000);
      return () => clearInterval(interval);
    } else {
      setBalance(null);
      setError(null);
    }
  }, [isConnected, refreshBalance]);

  const value: EnhancedWalletContextType = {
    isConnected,
    isConnecting,
    balance,
    balanceLoading,
    error,
    refreshBalance,
  };

  return (
    <EnhancedWalletContext.Provider value={value}>
      {children}
    </EnhancedWalletContext.Provider>
  );
}
