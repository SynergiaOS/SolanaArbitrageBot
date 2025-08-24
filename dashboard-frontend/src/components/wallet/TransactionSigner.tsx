"use client";

import React, { useState } from 'react';
import { useWallet, useConnection } from '@solana/wallet-adapter-react';
import { Transaction, VersionedTransaction } from '@solana/web3.js';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { 
  AlertTriangle, 
  CheckCircle, 
  Clock, 
  ExternalLink,
  Loader2,
  X
} from 'lucide-react';

interface TransactionSignerProps {
  transactionData?: string; // Base64 encoded transaction
  onSuccess?: (signature: string) => void;
  onError?: (error: string) => void;
  onCancel?: () => void;
  metadata?: {
    type: 'snipe' | 'sell' | 'swap';
    tokenSymbol?: string;
    amount?: string;
    estimatedGas?: number;
  };
}

export function TransactionSigner({
  transactionData,
  onSuccess,
  onError,
  onCancel,
  metadata
}: TransactionSignerProps) {
  const { publicKey, signTransaction, signAllTransactions } = useWallet();
  const { connection } = useConnection();
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSign = async () => {
    if (!publicKey || !signTransaction || !transactionData) {
      setError('Wallet not connected or transaction data missing');
      return;
    }

    setIsProcessing(true);
    setError(null);

    try {
      // Decode transaction from base64
      const transactionBuffer = Buffer.from(transactionData, 'base64');
      
      // Try to deserialize as VersionedTransaction first, then legacy Transaction
      let transaction: Transaction | VersionedTransaction;
      try {
        transaction = VersionedTransaction.deserialize(transactionBuffer);
      } catch {
        transaction = Transaction.from(transactionBuffer);
      }

      // Sign the transaction
      const signedTransaction = await signTransaction(transaction);

      // Send the transaction
      const signature = await connection.sendRawTransaction(
        signedTransaction.serialize(),
        {
          skipPreflight: false,
          preflightCommitment: 'confirmed',
        }
      );

      // Wait for confirmation
      const confirmation = await connection.confirmTransaction(signature, 'confirmed');
      
      if (confirmation.value.err) {
        throw new Error(`Transaction failed: ${confirmation.value.err}`);
      }

      onSuccess?.(signature);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Transaction failed';
      setError(errorMessage);
      onError?.(errorMessage);
    } finally {
      setIsProcessing(false);
    }
  };

  const getTransactionTypeIcon = () => {
    switch (metadata?.type) {
      case 'snipe':
        return '🎯';
      case 'sell':
        return '💰';
      case 'swap':
        return '🔄';
      default:
        return '📝';
    }
  };

  const getTransactionTypeColor = () => {
    switch (metadata?.type) {
      case 'snipe':
        return 'bg-green-500';
      case 'sell':
        return 'bg-blue-500';
      case 'swap':
        return 'bg-purple-500';
      default:
        return 'bg-gray-500';
    }
  };

  if (!transactionData) {
    return (
      <Card className="w-full max-w-md">
        <CardContent className="p-6 text-center">
          <Clock className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
          <p className="text-muted-foreground">Waiting for transaction...</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="w-full max-w-md">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <span className="text-2xl">{getTransactionTypeIcon()}</span>
            Transaction Approval
          </CardTitle>
          {onCancel && (
            <Button
              variant="ghost"
              size="icon"
              onClick={onCancel}
              disabled={isProcessing}
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Transaction Type */}
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">Type:</span>
          <Badge className={getTransactionTypeColor()}>
            {metadata?.type?.toUpperCase() || 'TRANSACTION'}
          </Badge>
        </div>

        {/* Token Info */}
        {metadata?.tokenSymbol && (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Token:</span>
            <span className="font-medium">{metadata.tokenSymbol}</span>
          </div>
        )}

        {/* Amount */}
        {metadata?.amount && (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Amount:</span>
            <span className="font-medium">{metadata.amount}</span>
          </div>
        )}

        {/* Estimated Gas */}
        {metadata?.estimatedGas && (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Est. Gas:</span>
            <span className="text-sm">{metadata.estimatedGas} SOL</span>
          </div>
        )}

        {/* Warning */}
        <div className="flex items-start gap-2 p-3 bg-yellow-50 border border-yellow-200 rounded-lg">
          <AlertTriangle className="h-4 w-4 text-yellow-600 mt-0.5 flex-shrink-0" />
          <div className="text-xs text-yellow-800">
            <p className="font-medium mb-1">Review carefully</p>
            <p>This transaction will be executed immediately after signing. Make sure all details are correct.</p>
          </div>
        </div>

        {/* Error Display */}
        {error && (
          <div className="p-3 bg-red-50 border border-red-200 rounded-lg">
            <div className="flex items-start gap-2">
              <X className="h-4 w-4 text-red-600 mt-0.5 flex-shrink-0" />
              <div className="text-xs text-red-800">
                <p className="font-medium mb-1">Transaction Failed</p>
                <p>{error}</p>
              </div>
            </div>
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex gap-2 pt-2">
          {onCancel && (
            <Button
              variant="outline"
              onClick={onCancel}
              disabled={isProcessing}
              className="flex-1"
            >
              Cancel
            </Button>
          )}
          <Button
            onClick={handleSign}
            disabled={isProcessing || !publicKey}
            className="flex-1"
          >
            {isProcessing ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Signing...
              </>
            ) : (
              <>
                <CheckCircle className="h-4 w-4 mr-2" />
                Sign & Send
              </>
            )}
          </Button>
        </div>

        {/* Wallet Status */}
        {!publicKey && (
          <div className="text-center text-sm text-muted-foreground">
            Please connect your wallet to sign transactions
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// Success component for completed transactions
interface TransactionSuccessProps {
  signature: string;
  onClose?: () => void;
}

export function TransactionSuccess({ signature, onClose }: TransactionSuccessProps) {
  const openInExplorer = () => {
    window.open(`https://solscan.io/tx/${signature}`, '_blank');
  };

  return (
    <Card className="w-full max-w-md">
      <CardContent className="p-6 text-center">
        <CheckCircle className="h-12 w-12 mx-auto mb-4 text-green-500" />
        <h3 className="font-semibold mb-2">Transaction Successful!</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Your transaction has been confirmed on the blockchain.
        </p>
        
        <div className="flex gap-2">
          <Button
            variant="outline"
            onClick={openInExplorer}
            className="flex-1"
          >
            <ExternalLink className="h-4 w-4 mr-2" />
            View on Explorer
          </Button>
          {onClose && (
            <Button onClick={onClose} className="flex-1">
              Close
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
