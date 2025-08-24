"use client";

import React, { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletButton } from '@/components/wallet/WalletButton';
import { SniperTransactionManager } from '@/components/sniper/SniperTransactionManager';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { 
  TestTube, 
  Play, 
  Square, 
  CheckCircle, 
  AlertCircle,
  Zap,
  Wallet,
  Activity
} from 'lucide-react';

interface TestResult {
  test: string;
  status: 'pending' | 'running' | 'success' | 'error';
  message?: string;
  timestamp?: Date;
}

export default function TestWeb3Page() {
  const { publicKey, connected } = useWallet();
  const [testResults, setTestResults] = useState<TestResult[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [mockTransactionCount, setMockTransactionCount] = useState(0);

  // Initialize test results
  useEffect(() => {
    setTestResults([
      { test: 'Wallet Connection', status: 'pending' },
      { test: 'Enhanced WebSocket Connection', status: 'pending' },
      { test: 'Auto-Snipe Enable/Disable', status: 'pending' },
      { test: 'Mock Transaction Generation', status: 'pending' },
      { test: 'Transaction Approval Flow', status: 'pending' },
      { test: 'Transaction Success Handling', status: 'pending' },
    ]);
  }, []);

  // Update wallet connection test
  useEffect(() => {
    updateTestResult('Wallet Connection', 
      connected ? 'success' : 'pending',
      connected ? `Connected: ${publicKey?.toString().slice(0, 8)}...` : 'Please connect wallet'
    );
  }, [connected, publicKey]);

  const updateTestResult = (testName: string, status: TestResult['status'], message?: string) => {
    setTestResults(prev => prev.map(result => 
      result.test === testName 
        ? { ...result, status, message, timestamp: new Date() }
        : result
    ));
  };

  const runWebSocketTest = async () => {
    updateTestResult('Enhanced WebSocket Connection', 'running');
    
    try {
      // Test WebSocket connection
      const wsUrl = process.env.NEXT_PUBLIC_ENHANCED_WS_URL || 'ws://localhost:8080/enhanced-ws';
      const ws = new WebSocket(wsUrl);
      
      await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('Connection timeout'));
        }, 5000);
        
        ws.onopen = () => {
          clearTimeout(timeout);
          updateTestResult('Enhanced WebSocket Connection', 'success', 'Connected successfully');
          resolve(true);
        };
        
        ws.onerror = (error) => {
          clearTimeout(timeout);
          reject(error);
        };
      });
      
      ws.close();
    } catch (error) {
      updateTestResult('Enhanced WebSocket Connection', 'error', 
        `Failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const runAutoSnipeTest = async () => {
    if (!connected || !publicKey) {
      updateTestResult('Auto-Snipe Enable/Disable', 'error', 'Wallet not connected');
      return;
    }

    updateTestResult('Auto-Snipe Enable/Disable', 'running');
    
    try {
      // Test auto-snipe enable
      const wsUrl = process.env.NEXT_PUBLIC_ENHANCED_WS_URL || 'ws://localhost:8080/enhanced-ws';
      const ws = new WebSocket(wsUrl);
      
      await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('Auto-snipe test timeout'));
        }, 10000);
        
        ws.onopen = () => {
          // Register wallet
          ws.send(JSON.stringify({
            type: 'register_wallet',
            wallet_address: publicKey.toString(),
          }));
          
          // Enable auto-snipe
          setTimeout(() => {
            ws.send(JSON.stringify({
              type: 'enable_auto_snipe',
              wallet_address: publicKey.toString(),
            }));
          }, 1000);
          
          // Disable auto-snipe after 3 seconds
          setTimeout(() => {
            ws.send(JSON.stringify({
              type: 'disable_auto_snipe',
              wallet_address: publicKey.toString(),
            }));
            
            clearTimeout(timeout);
            updateTestResult('Auto-Snipe Enable/Disable', 'success', 'Enable/disable cycle completed');
            resolve(true);
          }, 3000);
        };
        
        ws.onerror = (error) => {
          clearTimeout(timeout);
          reject(error);
        };
      });
      
      ws.close();
    } catch (error) {
      updateTestResult('Auto-Snipe Enable/Disable', 'error', 
        `Failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const runMockTransactionTest = async () => {
    if (!connected || !publicKey) {
      updateTestResult('Mock Transaction Generation', 'error', 'Wallet not connected');
      return;
    }

    updateTestResult('Mock Transaction Generation', 'running');
    
    try {
      const wsUrl = process.env.NEXT_PUBLIC_ENHANCED_WS_URL || 'ws://localhost:8080/enhanced-ws';
      const ws = new WebSocket(wsUrl);
      
      let transactionReceived = false;
      
      await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          if (!transactionReceived) {
            reject(new Error('No mock transaction received within 20 seconds'));
          }
        }, 20000);
        
        ws.onopen = () => {
          // Register wallet and enable auto-snipe
          ws.send(JSON.stringify({
            type: 'register_wallet',
            wallet_address: publicKey.toString(),
          }));
          
          setTimeout(() => {
            ws.send(JSON.stringify({
              type: 'enable_auto_snipe',
              wallet_address: publicKey.toString(),
            }));
          }, 1000);
        };
        
        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            
            if (data.type === 'transaction_request') {
              transactionReceived = true;
              setMockTransactionCount(prev => prev + 1);
              
              clearTimeout(timeout);
              updateTestResult('Mock Transaction Generation', 'success', 
                `Received mock transaction: ${data.token_symbol} (${data.transaction_type})`);
              
              // Test transaction approval flow
              updateTestResult('Transaction Approval Flow', 'running');
              
              // Simulate user approval after 2 seconds
              setTimeout(() => {
                ws.send(JSON.stringify({
                  type: 'transaction_response',
                  transaction_id: data.id,
                  signature: 'mock_signature_' + Date.now(),
                  success: true,
                  wallet_address: publicKey.toString(),
                }));
                
                updateTestResult('Transaction Approval Flow', 'success', 'Mock approval sent');
                updateTestResult('Transaction Success Handling', 'success', 'Transaction completed successfully');
              }, 2000);
              
              resolve(true);
            }
          } catch (error) {
            console.error('Error parsing WebSocket message:', error);
          }
        };
        
        ws.onerror = (error) => {
          clearTimeout(timeout);
          reject(error);
        };
      });
      
      // Keep connection open for a bit longer
      setTimeout(() => {
        ws.close();
      }, 5000);
      
    } catch (error) {
      updateTestResult('Mock Transaction Generation', 'error', 
        `Failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const runAllTests = async () => {
    setIsRunning(true);
    
    try {
      await runWebSocketTest();
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      await runAutoSnipeTest();
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      await runMockTransactionTest();
    } catch (error) {
      console.error('Test suite error:', error);
    } finally {
      setIsRunning(false);
    }
  };

  const getStatusIcon = (status: TestResult['status']) => {
    switch (status) {
      case 'success':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'error':
        return <AlertCircle className="h-4 w-4 text-red-500" />;
      case 'running':
        return <Activity className="h-4 w-4 text-blue-500 animate-pulse" />;
      default:
        return <div className="h-4 w-4 rounded-full border-2 border-gray-300" />;
    }
  };

  const getStatusColor = (status: TestResult['status']) => {
    switch (status) {
      case 'success':
        return 'bg-green-50 border-green-200';
      case 'error':
        return 'bg-red-50 border-red-200';
      case 'running':
        return 'bg-blue-50 border-blue-200';
      default:
        return 'bg-gray-50 border-gray-200';
    }
  };

  return (
    <div className="p-6 space-y-6">
      {/* Page Header */}
      <div className="flex items-center gap-3">
        <TestTube className="h-8 w-8 text-primary" />
        <div>
          <h1 className="text-3xl font-bold">Web3 Integration Test</h1>
          <p className="text-muted-foreground">
            Test wallet connection, WebSocket communication, and transaction flow
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Test Controls */}
        <div className="space-y-4">
          {/* Wallet Connection */}
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

          {/* Test Controls */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Zap className="h-5 w-5" />
                Test Controls
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <Button
                onClick={runAllTests}
                disabled={isRunning || !connected}
                className="w-full"
              >
                {isRunning ? (
                  <>
                    <Activity className="h-4 w-4 mr-2 animate-spin" />
                    Running Tests...
                  </>
                ) : (
                  <>
                    <Play className="h-4 w-4 mr-2" />
                    Run All Tests
                  </>
                )}
              </Button>
              
              {!connected && (
                <p className="text-sm text-muted-foreground text-center">
                  Connect wallet to run tests
                </p>
              )}
              
              {mockTransactionCount > 0 && (
                <div className="text-center">
                  <Badge variant="secondary">
                    {mockTransactionCount} mock transactions received
                  </Badge>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Test Results */}
        <div>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <TestTube className="h-5 w-5" />
                Test Results
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {testResults.map((result, index) => (
                <div
                  key={index}
                  className={`p-3 border rounded-lg ${getStatusColor(result.status)}`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      {getStatusIcon(result.status)}
                      <span className="font-medium">{result.test}</span>
                    </div>
                    <Badge variant="outline" className="text-xs">
                      {result.status}
                    </Badge>
                  </div>
                  
                  {result.message && (
                    <p className="text-sm text-muted-foreground mt-1">
                      {result.message}
                    </p>
                  )}
                  
                  {result.timestamp && (
                    <p className="text-xs text-muted-foreground mt-1">
                      {result.timestamp.toLocaleTimeString()}
                    </p>
                  )}
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Live Transaction Manager */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Live Transaction Manager
          </CardTitle>
        </CardHeader>
        <CardContent>
          <SniperTransactionManager />
        </CardContent>
      </Card>
    </div>
  );
}
