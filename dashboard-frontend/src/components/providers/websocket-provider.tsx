"use client"

import React, { createContext, useContext, useEffect, useState, useRef } from 'react'
import {
  mockArbitrageOpportunities,
  mockSniperActivities,
  mockPositions,
  mockPerformanceMetrics,
  mockGepaMetrics,
  mockKestraMetrics,
  mockSystemHealth,
  generateMockData
} from '@/lib/mock-data'

// Types for the enhanced bot data
export interface ArbitrageOpportunity {
  id: string
  tokenA: string
  tokenB: string
  dexA: string
  dexB: string
  priceA: number
  priceB: number
  spread: number
  profitUsd: number
  timestamp: number
}

export interface SniperActivity {
  id: string
  tokenMint: string
  tokenName?: string
  tokenSymbol?: string
  action: 'detected' | 'analyzing' | 'sniped' | 'rejected'
  riskScore: number
  liquiditySol: number
  marketCapUsd: number
  timestamp: number
  executionTimeMs?: number
  profitMultiplier?: number
}

export interface Position {
  id: string
  tokenMint: string
  tokenName?: string
  tokenSymbol?: string
  strategy: 'arbitrage' | 'sniper'
  entryPrice: number
  currentPrice: number
  positionSizeSol: number
  unrealizedPnl: number
  realizedPnl: number
  multiplier: number
  timestamp: number
}

export interface PerformanceMetrics {
  totalTrades: number
  successfulTrades: number
  totalProfitUsd: number
  totalLossUsd: number
  winRate: number
  sharpeRatio: number
  maxDrawdown: number
  dailyRoi: number
  monthlyRoi: number
}

export interface GepaMetrics {
  currentGeneration: number
  maxGenerations: number
  bestFitnessScore: number
  populationDiversity: number
  isOptimizing: boolean
  lastOptimization: number
  improvementPercent: number
}

export interface KestraMetrics {
  activeWorkflows: number
  completedWorkflows: number
  failedWorkflows: number
  successRate: number
  lastExecution: number
  uptime: number
}

export interface SystemHealth {
  botStatus: 'online' | 'offline' | 'error'
  rpcStatus: 'connected' | 'disconnected' | 'slow'
  walletBalance: number
  systemLoad: number
  memoryUsage: number
  uptime: number
  lastUpdate: number
}

export interface WebSocketData {
  type: 'arbitrage' | 'sniper' | 'position' | 'performance' | 'gepa' | 'kestra' | 'health'
  data: any
  timestamp: number
}

interface WebSocketContextType {
  isConnected: boolean
  arbitrageOpportunities: ArbitrageOpportunity[]
  sniperActivities: SniperActivity[]
  positions: Position[]
  performanceMetrics: PerformanceMetrics | null
  gepaMetrics: GepaMetrics | null
  kestraMetrics: KestraMetrics | null
  systemHealth: SystemHealth | null
  sendMessage: (message: any) => void
  reconnect: () => void
}

const WebSocketContext = createContext<WebSocketContextType | undefined>(undefined)

export function WebSocketProvider({ children }: { children: React.ReactNode }) {
  const [isConnected, setIsConnected] = useState(false)
  const [useMockData, setUseMockData] = useState(false)
  const [arbitrageOpportunities, setArbitrageOpportunities] = useState<ArbitrageOpportunity[]>([])
  const [sniperActivities, setSniperActivities] = useState<SniperActivity[]>([])
  const [positions, setPositions] = useState<Position[]>([])
  const [performanceMetrics, setPerformanceMetrics] = useState<PerformanceMetrics | null>(null)
  const [gepaMetrics, setGepaMetrics] = useState<GepaMetrics | null>(null)
  const [kestraMetrics, setKestraMetrics] = useState<KestraMetrics | null>(null)
  const [systemHealth, setSystemHealth] = useState<SystemHealth | null>(null)
  
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const reconnectAttempts = useRef(0)

  const connect = () => {
    try {
      // Connect to the enhanced bot's WebSocket endpoint
      const wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8080/ws'
      console.log('Attempting to connect to WebSocket:', wsUrl)

      wsRef.current = new WebSocket(wsUrl)

      wsRef.current.onopen = () => {
        console.log('WebSocket connected to:', wsUrl)
        setIsConnected(true)
        reconnectAttempts.current = 0

        // Request initial data
        sendMessage({ type: 'subscribe', channels: ['all'] })
      }

      wsRef.current.onmessage = (event) => {
        try {
          const message: WebSocketData = JSON.parse(event.data)
          handleMessage(message)
        } catch (error) {
          console.error('Error parsing WebSocket message:', error)
        }
      }

      wsRef.current.onclose = (event) => {
        console.log('WebSocket disconnected:', event.code, event.reason)
        setIsConnected(false)

        // Only attempt to reconnect if it wasn't a manual close
        if (event.code !== 1000 && reconnectAttempts.current < 10) {
          const delay = Math.min(1000 * Math.pow(2, reconnectAttempts.current), 30000)
          console.log(`Reconnecting in ${delay}ms (attempt ${reconnectAttempts.current + 1}/10)`)
          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectAttempts.current++
            connect()
          }, delay)
        } else if (reconnectAttempts.current >= 10) {
          console.error('Max reconnection attempts reached. Switching to mock data mode.')
          console.log('To connect to real data, ensure the Enhanced Bot is running on port 8080')
          setUseMockData(true)
          initializeMockData()
        }
      }

      wsRef.current.onerror = (error) => {
        console.error('WebSocket error:', error)
        console.error('Make sure the Enhanced Solana Arbitrage Bot is running on port 8080')
      }
    } catch (error) {
      console.error('Error connecting to WebSocket:', error)
      console.error('Please ensure the Enhanced Bot is running and accessible')
    }
  }

  const handleMessage = (message: WebSocketData) => {
    switch (message.type) {
      case 'arbitrage':
        setArbitrageOpportunities(prev => {
          const updated = [message.data, ...prev.slice(0, 49)] // Keep last 50
          return updated
        })
        break
        
      case 'sniper':
        setSniperActivities(prev => {
          const updated = [message.data, ...prev.slice(0, 99)] // Keep last 100
          return updated
        })
        break
        
      case 'position':
        setPositions(prev => {
          const existingIndex = prev.findIndex(p => p.id === message.data.id)
          if (existingIndex >= 0) {
            const updated = [...prev]
            updated[existingIndex] = message.data
            return updated
          } else {
            return [message.data, ...prev]
          }
        })
        break
        
      case 'performance':
        setPerformanceMetrics(message.data)
        break
        
      case 'gepa':
        setGepaMetrics(message.data)
        break
        
      case 'kestra':
        setKestraMetrics(message.data)
        break
        
      case 'health':
        setSystemHealth(message.data)
        break
        
      default:
        console.log('Unknown message type:', message.type)
    }
  }

  const sendMessage = (message: any) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message))
    }
  }

  const initializeMockData = () => {
    console.log('🎭 Initializing mock data for development...')
    setArbitrageOpportunities(mockArbitrageOpportunities)
    setSniperActivities(mockSniperActivities)
    setPositions(mockPositions)
    setPerformanceMetrics(mockPerformanceMetrics)
    setGepaMetrics(mockGepaMetrics)
    setKestraMetrics(mockKestraMetrics)
    setSystemHealth(mockSystemHealth)

    // Simulate periodic updates with mock data
    const mockInterval = setInterval(() => {
      const newData = generateMockData()
      setArbitrageOpportunities(prev => [newData.arbitrageOpportunities[0], ...prev.slice(0, 9)])
      setSniperActivities(prev => [newData.sniperActivities[0], ...prev.slice(0, 9)])
      setPerformanceMetrics(newData.performanceMetrics)
      setSystemHealth(newData.systemHealth)
    }, 5000)

    return () => clearInterval(mockInterval)
  }

  const reconnect = () => {
    if (wsRef.current) {
      wsRef.current.close()
    }
    setUseMockData(false)
    reconnectAttempts.current = 0
    connect()
  }

  useEffect(() => {
    // Try to connect to real WebSocket first
    connect()

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
  }, [])

  // Initialize mock data immediately for development
  useEffect(() => {
    if (!isConnected && !useMockData) {
      const timer = setTimeout(() => {
        if (!isConnected) {
          console.log('🎭 No connection established, using mock data for development')
          setUseMockData(true)
          initializeMockData()
        }
      }, 3000) // Wait 3 seconds before falling back to mock data

      return () => clearTimeout(timer)
    }
  }, [isConnected, useMockData])

  const value: WebSocketContextType = {
    isConnected,
    arbitrageOpportunities,
    sniperActivities,
    positions,
    performanceMetrics,
    gepaMetrics,
    kestraMetrics,
    systemHealth,
    sendMessage,
    reconnect,
  }

  return (
    <WebSocketContext.Provider value={value}>
      {children}
    </WebSocketContext.Provider>
  )
}

export function useWebSocket() {
  const context = useContext(WebSocketContext)
  if (context === undefined) {
    throw new Error('useWebSocket must be used within a WebSocketProvider')
  }
  return context
}
