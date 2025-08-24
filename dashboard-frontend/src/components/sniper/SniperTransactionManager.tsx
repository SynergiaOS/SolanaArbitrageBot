"use client";

import React, { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { useWebSocket } from '@/components/providers/websocket-provider';
import { TransactionSigner, TransactionSuccess } from '@/components/wallet/TransactionSigner';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { 
  Target, 
  DollarSign, 
  Clock, 
  TrendingUp,
  AlertTriangle,
  CheckCircle,
  X
} from 'lucide-react';

interface PendingTransaction {
  id: string;
  type: 'snipe' | 'sell';
  tokenSymbol: string;
  tokenMint: string;
  amount: string;
  transactionData: string;
  timestamp: number;
  metadata?: {
    estimatedGas?: number;
    priceImpact?: number;
    slippage?: number;
  };
}

interface CompletedTransaction {
  id: string;
  signature: string;
  type: 'snipe' | 'sell';
  tokenSymbol: string;
  success: boolean;
  timestamp: number;
}

interface ArbitrageOpportunity {
  id: string;
  buy_dex: string;
  sell_dex: string;
  buy_price: number;
  sell_price: number;
  profit_usd: number;
  profit_percentage: number;
  confidence_score: number;
  amount_sol: number;
  timestamp: string;
}

export function SniperTransactionManager() {
  const { publicKey } = useWallet();
  const { sendMessage } = useWebSocket();
  const [pendingTransactions, setPendingTransactions] = useState<PendingTransaction[]>([]);
  const [completedTransactions, setCompletedTransactions] = useState<CompletedTransaction[]>([]);
  const [currentTransaction, setCurrentTransaction] = useState<PendingTransaction | null>(null);
  const [showSuccess, setShowSuccess] = useState<string | null>(null);
  const [recentOpportunities, setRecentOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [liveOpportunityCount, setLiveOpportunityCount] = useState(0);

  // Listen for transaction requests from backend
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);
        
        if (data.type === 'transaction_request') {
          const transaction: PendingTransaction = {
            id: data.id,
            type: data.transactionType,
            tokenSymbol: data.tokenSymbol,
            tokenMint: data.tokenMint,
            amount: data.amount,
            transactionData: data.transactionData,
            timestamp: Date.now(),
            metadata: data.metadata,
          };

          setPendingTransactions(prev => [...prev, transaction]);

          // Auto-select if no current transaction
          if (!currentTransaction) {
            setCurrentTransaction(transaction);
          }
        } else if (data.type === 'arbitrage_opportunity') {
          const opportunity: ArbitrageOpportunity = {
            id: data.id,
            buy_dex: data.buy_dex,
            sell_dex: data.sell_dex,
            buy_price: data.buy_price,
            sell_price: data.sell_price,
            profit_usd: data.profit_usd,
            profit_percentage: data.profit_percentage,
            confidence_score: data.confidence_score,
            amount_sol: data.amount_sol,
            timestamp: data.timestamp,
          };

          // Add to recent opportunities (keep last 10)
          setRecentOpportunities(prev => {
            const updated = [opportunity, ...prev].slice(0, 10);
            return updated;
          });

          // Update live count
          setLiveOpportunityCount(prev => prev + 1);
        }
      } catch (error) {
        console.error('Error parsing transaction message:', error);
      }
    };

    // This would be connected to the WebSocket in a real implementation
    // For now, we'll simulate with a custom event
    window.addEventListener('sniper-transaction', handleMessage as any);
    
    return () => {
      window.removeEventListener('sniper-transaction', handleMessage as any);
    };
  }, [currentTransaction]);

  const handleTransactionSuccess = (signature: string) => {
    if (!currentTransaction) return;

    // Add to completed transactions
    const completed: CompletedTransaction = {
      id: currentTransaction.id,
      signature,
      type: currentTransaction.type,
      tokenSymbol: currentTransaction.tokenSymbol,
      success: true,
      timestamp: Date.now(),
    };
    
    setCompletedTransactions(prev => [completed, ...prev.slice(0, 9)]); // Keep last 10
    
    // Remove from pending
    setPendingTransactions(prev => prev.filter(tx => tx.id !== currentTransaction.id));
    
    // Show success
    setShowSuccess(signature);
    setCurrentTransaction(null);
    
    // Notify backend
    sendMessage({
      type: 'transaction_completed',
      transactionId: currentTransaction.id,
      signature,
      success: true,
    });
    
    // Auto-select next pending transaction
    const nextTransaction = pendingTransactions.find(tx => tx.id !== currentTransaction.id);
    if (nextTransaction) {
      setTimeout(() => setCurrentTransaction(nextTransaction), 1000);
    }
  };

  const handleTransactionError = (error: string) => {
    if (!currentTransaction) return;

    // Add to completed transactions as failed
    const completed: CompletedTransaction = {
      id: currentTransaction.id,
      signature: '',
      type: currentTransaction.type,
      tokenSymbol: currentTransaction.tokenSymbol,
      success: false,
      timestamp: Date.now(),
    };
    
    setCompletedTransactions(prev => [completed, ...prev.slice(0, 9)]);
    
    // Remove from pending
    setPendingTransactions(prev => prev.filter(tx => tx.id !== currentTransaction.id));
    
    // Notify backend
    sendMessage({
      type: 'transaction_completed',
      transactionId: currentTransaction.id,
      signature: '',
      success: false,
      error,
    });
    
    setCurrentTransaction(null);
  };

  const handleTransactionCancel = () => {
    if (!currentTransaction) return;

    // Remove from pending
    setPendingTransactions(prev => prev.filter(tx => tx.id !== currentTransaction.id));
    
    // Notify backend
    sendMessage({
      type: 'transaction_cancelled',
      transactionId: currentTransaction.id,
    });
    
    setCurrentTransaction(null);
  };

  const selectTransaction = (transaction: PendingTransaction) => {
    setCurrentTransaction(transaction);
    setShowSuccess(null);
  };

  const enableAutoSnipe = () => {
    sendMessage({
      type: 'enable_auto_snipe',
      walletAddress: publicKey?.toString(),
    });
  };

  const disableAutoSnipe = () => {
    sendMessage({
      type: 'disable_auto_snipe',
    });
  };

  if (showSuccess) {
    return (
      <div className="space-y-4">
        <TransactionSuccess
          signature={showSuccess}
          onClose={() => setShowSuccess(null)}
        />
        {pendingTransactions.length > 0 && (
          <Card>
            <CardContent className="p-4">
              <p className="text-sm text-center">
                {pendingTransactions.length} more transaction{pendingTransactions.length > 1 ? 's' : ''} pending
              </p>
            </CardContent>
          </Card>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Live Arbitrage Opportunities */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2">
            <TrendingUp className="h-5 w-5" />
            Live Arbitrage Opportunities
            <Badge variant="secondary" className="ml-auto">
              {liveOpportunityCount} found
            </Badge>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {recentOpportunities.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">
              Waiting for arbitrage opportunities...
            </p>
          ) : (
            <div className="space-y-2 max-h-48 overflow-y-auto">
              {recentOpportunities.map((opportunity) => (
                <div
                  key={opportunity.id}
                  className="flex items-center justify-between p-3 border rounded-lg bg-muted/50"
                >
                  <div className="flex-1">
                    <div className="flex items-center gap-2 text-sm font-medium">
                      <span className="text-blue-600">{opportunity.buy_dex}</span>
                      <span>→</span>
                      <span className="text-green-600">{opportunity.sell_dex}</span>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      ${opportunity.buy_price.toFixed(4)} → ${opportunity.sell_price.toFixed(4)}
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-sm font-medium text-green-600">
                      +${opportunity.profit_usd.toFixed(2)}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {opportunity.profit_percentage.toFixed(2)}%
                    </div>
                  </div>
                  <div className="ml-3">
                    <Badge
                      variant={opportunity.confidence_score > 0.8 ? "default" : "secondary"}
                      className="text-xs"
                    >
                      {(opportunity.confidence_score * 100).toFixed(0)}%
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Auto-Snipe Controls */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2">
            <Target className="h-5 w-5" />
            Sniper Controls
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex gap-2">
            <Button
              onClick={enableAutoSnipe}
              disabled={!publicKey}
              className="flex-1"
              variant="default"
            >
              Enable Auto-Snipe
            </Button>
            <Button
              onClick={disableAutoSnipe}
              variant="outline"
              className="flex-1"
            >
              Disable Auto-Snipe
            </Button>
          </div>
          {!publicKey && (
            <p className="text-xs text-muted-foreground text-center">
              Connect wallet to enable auto-sniping
            </p>
          )}
        </CardContent>
      </Card>

      {/* Current Transaction */}
      {currentTransaction && (
        <TransactionSigner
          transactionData={currentTransaction.transactionData}
          onSuccess={handleTransactionSuccess}
          onError={handleTransactionError}
          onCancel={handleTransactionCancel}
          metadata={{
            type: currentTransaction.type,
            tokenSymbol: currentTransaction.tokenSymbol,
            amount: currentTransaction.amount,
            estimatedGas: currentTransaction.metadata?.estimatedGas,
          }}
        />
      )}

      {/* Pending Transactions Queue */}
      {pendingTransactions.length > 0 && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center justify-between">
              <span className="flex items-center gap-2">
                <Clock className="h-5 w-5" />
                Pending Transactions
              </span>
              <Badge variant="secondary">
                {pendingTransactions.length}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {pendingTransactions.map((tx) => (
              <div
                key={tx.id}
                className={`p-3 border rounded-lg cursor-pointer transition-colors ${
                  currentTransaction?.id === tx.id
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:bg-muted/50'
                }`}
                onClick={() => selectTransaction(tx)}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    {tx.type === 'snipe' ? (
                      <Target className="h-4 w-4 text-green-500" />
                    ) : (
                      <DollarSign className="h-4 w-4 text-blue-500" />
                    )}
                    <span className="font-medium">{tx.tokenSymbol}</span>
                    <Badge variant="outline" className="text-xs">
                      {tx.type.toUpperCase()}
                    </Badge>
                  </div>
                  <span className="text-sm text-muted-foreground">
                    {tx.amount}
                  </span>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Recent Transactions */}
      {completedTransactions.length > 0 && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5" />
              Recent Transactions
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {completedTransactions.slice(0, 5).map((tx) => (
              <div
                key={tx.id}
                className="flex items-center justify-between p-2 border rounded"
              >
                <div className="flex items-center gap-2">
                  {tx.success ? (
                    <CheckCircle className="h-4 w-4 text-green-500" />
                  ) : (
                    <X className="h-4 w-4 text-red-500" />
                  )}
                  <span className="text-sm">{tx.tokenSymbol}</span>
                  <Badge variant="outline" className="text-xs">
                    {tx.type.toUpperCase()}
                  </Badge>
                </div>
                <div className="text-xs text-muted-foreground">
                  {new Date(tx.timestamp).toLocaleTimeString()}
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Empty State */}
      {pendingTransactions.length === 0 && completedTransactions.length === 0 && (
        <Card>
          <CardContent className="p-8 text-center">
            <Target className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
            <h3 className="font-medium mb-2">No Transactions</h3>
            <p className="text-sm text-muted-foreground">
              Sniper transactions will appear here when opportunities are detected.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
