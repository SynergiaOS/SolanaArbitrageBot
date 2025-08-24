"use client";

import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { useWebSocket } from './websocket-provider';

interface TransactionRequest {
  id: string;
  type: 'snipe' | 'sell' | 'swap';
  tokenSymbol: string;
  tokenMint: string;
  amount: string;
  transactionData: string; // Base64 encoded transaction
  metadata?: {
    estimatedGas?: number;
    priceImpact?: number;
    slippage?: number;
  };
}

interface EnhancedWebSocketContextType {
  pendingTransactions: TransactionRequest[];
  sendTransactionResponse: (transactionId: string, signature: string, success: boolean, error?: string) => void;
  enableAutoSnipe: (walletAddress: string) => void;
  disableAutoSnipe: () => void;
}

const EnhancedWebSocketContext = createContext<EnhancedWebSocketContextType>({
  pendingTransactions: [],
  sendTransactionResponse: () => {},
  enableAutoSnipe: () => {},
  disableAutoSnipe: () => {},
});

export function useEnhancedWebSocket() {
  return useContext(EnhancedWebSocketContext);
}

interface EnhancedWebSocketProviderProps {
  children: React.ReactNode;
}

export function EnhancedWebSocketProvider({ children }: EnhancedWebSocketProviderProps) {
  const { publicKey } = useWallet();
  const { sendMessage } = useWebSocket();
  const [pendingTransactions, setPendingTransactions] = useState<TransactionRequest[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  // Connect to enhanced WebSocket for transaction requests
  useEffect(() => {
    const connectEnhancedWS = () => {
      const wsUrl = process.env.NEXT_PUBLIC_ENHANCED_WS_URL || 'ws://localhost:8080/enhanced-ws';
      
      try {
        wsRef.current = new WebSocket(wsUrl);
        
        wsRef.current.onopen = () => {
          console.log('🔗 Enhanced WebSocket connected for transaction handling');
          
          // Register wallet if connected
          if (publicKey) {
            wsRef.current?.send(JSON.stringify({
              type: 'register_wallet',
              walletAddress: publicKey.toString(),
            }));
          }
        };
        
        wsRef.current.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            
            switch (data.type) {
              case 'transaction_request':
                const transaction: TransactionRequest = {
                  id: data.id,
                  type: data.transactionType,
                  tokenSymbol: data.tokenSymbol,
                  tokenMint: data.tokenMint,
                  amount: data.amount,
                  transactionData: data.transactionData,
                  metadata: data.metadata,
                };
                
                setPendingTransactions(prev => [...prev, transaction]);
                
                // Dispatch custom event for SniperTransactionManager
                window.dispatchEvent(new CustomEvent('sniper-transaction', {
                  detail: data
                }));
                break;
                
              case 'transaction_cancelled':
                setPendingTransactions(prev => 
                  prev.filter(tx => tx.id !== data.transactionId)
                );
                break;
                
              default:
                console.log('Unknown enhanced WebSocket message:', data);
            }
          } catch (error) {
            console.error('Error parsing enhanced WebSocket message:', error);
          }
        };
        
        wsRef.current.onclose = () => {
          console.log('🔌 Enhanced WebSocket disconnected');
          // Attempt to reconnect after 5 seconds
          setTimeout(connectEnhancedWS, 5000);
        };
        
        wsRef.current.onerror = (error) => {
          console.error('Enhanced WebSocket error:', error);
        };
        
      } catch (error) {
        console.error('Failed to connect enhanced WebSocket:', error);
        // Retry connection after 5 seconds
        setTimeout(connectEnhancedWS, 5000);
      }
    };
    
    connectEnhancedWS();
    
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  // Register wallet when connected
  useEffect(() => {
    if (publicKey && wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'register_wallet',
        walletAddress: publicKey.toString(),
      }));
    }
  }, [publicKey]);

  const sendTransactionResponse = (
    transactionId: string, 
    signature: string, 
    success: boolean, 
    error?: string
  ) => {
    // Send to enhanced WebSocket
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'transaction_response',
        transactionId,
        signature,
        success,
        error,
        walletAddress: publicKey?.toString(),
      }));
    }
    
    // Also send to main WebSocket for dashboard updates
    sendMessage({
      type: 'transaction_completed',
      transactionId,
      signature,
      success,
      error,
    });
    
    // Remove from pending
    setPendingTransactions(prev => 
      prev.filter(tx => tx.id !== transactionId)
    );
  };

  const enableAutoSnipe = (walletAddress: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'enable_auto_snipe',
        walletAddress,
      }));
    }
    
    // Also notify main WebSocket
    sendMessage({
      type: 'enable_auto_snipe',
      walletAddress,
    });
  };

  const disableAutoSnipe = () => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'disable_auto_snipe',
        walletAddress: publicKey?.toString(),
      }));
    }
    
    // Also notify main WebSocket
    sendMessage({
      type: 'disable_auto_snipe',
    });
  };

  const value: EnhancedWebSocketContextType = {
    pendingTransactions,
    sendTransactionResponse,
    enableAutoSnipe,
    disableAutoSnipe,
  };

  return (
    <EnhancedWebSocketContext.Provider value={value}>
      {children}
    </EnhancedWebSocketContext.Provider>
  );
}
