"use client";

import { useWebSocket } from "@/components/providers/websocket-provider";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Brain, TrendingUp, Target, BarChart3, Zap, Settings } from "lucide-react";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

export default function GepaPage() {
  const { 
    isConnected, 
    gepaMetrics,
    performanceMetrics 
  } = useWebSocket();

  // Mock data for fitness progression chart
  const fitnessData = Array.from({ length: 20 }, (_, i) => ({
    generation: i + 1,
    fitness: 0.3 + (i * 0.035) + Math.random() * 0.1,
    diversity: 0.8 - (i * 0.03) + Math.random() * 0.1,
  }));

  // Mock parameter evolution data
  const parameterData = [
    { name: 'Position Size', current: 15.2, optimal: 18.5, improvement: '+21.7%' },
    { name: 'Profit Threshold', current: 0.8, optimal: 0.65, improvement: '+18.8%' },
    { name: 'Stop Loss', current: -8.5, optimal: -6.2, improvement: '+27.1%' },
    { name: 'Slippage Tolerance', current: 0.4, optimal: 0.55, improvement: '+37.5%' },
    { name: 'Risk Scaling', current: 1.2, optimal: 1.45, improvement: '+20.8%' },
  ];

  const formatPercentage = (value: number) => {
    return `${(value * 100).toFixed(1)}%`;
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground flex items-center gap-2">
            <Brain className="h-8 w-8" />
            GEPA Optimizer
          </h1>
          <p className="text-muted-foreground">
            Genetic Evolution Parameter Adaptation for optimal trading performance
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <div className={`w-3 h-3 rounded-full ${
            gepaMetrics?.isOptimizing ? 'bg-blue-500' : 
            isConnected ? 'bg-green-500' : 'bg-red-500'
          }`}></div>
          <span className="text-sm text-muted-foreground">
            {gepaMetrics?.isOptimizing ? 'Optimizing' : 
             isConnected ? 'Ready' : 'Disconnected'}
          </span>
        </div>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Current Generation</CardTitle>
            <Target className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {gepaMetrics ? `${gepaMetrics.currentGeneration}/${gepaMetrics.maxGenerations}` : '0/100'}
            </div>
            <p className="text-xs text-muted-foreground">
              Evolution progress
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Best Fitness</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {gepaMetrics ? gepaMetrics.bestFitnessScore.toFixed(3) : '0.000'}
            </div>
            <p className="text-xs text-muted-foreground">
              Highest score achieved
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Improvement</CardTitle>
            <Zap className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-600">
              {gepaMetrics ? `+${gepaMetrics.improvementPercent.toFixed(1)}%` : '+0.0%'}
            </div>
            <p className="text-xs text-muted-foreground">
              Performance gain
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Population Diversity</CardTitle>
            <BarChart3 className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {gepaMetrics ? formatPercentage(gepaMetrics.populationDiversity) : '0%'}
            </div>
            <p className="text-xs text-muted-foreground">
              Genetic variation
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Fitness Evolution Chart */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5" />
              Fitness Evolution
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={fitnessData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis 
                    dataKey="generation" 
                    className="text-xs fill-muted-foreground"
                  />
                  <YAxis 
                    className="text-xs fill-muted-foreground"
                    domain={[0, 1]}
                    tickFormatter={(value) => value.toFixed(1)}
                  />
                  <Tooltip 
                    formatter={(value: number, name: string) => [
                      value.toFixed(3), 
                      name === 'fitness' ? 'Fitness Score' : 'Population Diversity'
                    ]}
                    labelStyle={{ color: 'hsl(var(--foreground))' }}
                    contentStyle={{ 
                      backgroundColor: 'hsl(var(--background))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '6px'
                    }}
                  />
                  <Line 
                    type="monotone" 
                    dataKey="fitness" 
                    stroke="hsl(var(--primary))" 
                    strokeWidth={2}
                    dot={{ fill: 'hsl(var(--primary))', strokeWidth: 2, r: 3 }}
                    name="fitness"
                  />
                  <Line 
                    type="monotone" 
                    dataKey="diversity" 
                    stroke="hsl(var(--secondary))" 
                    strokeWidth={2}
                    strokeDasharray="5 5"
                    dot={{ fill: 'hsl(var(--secondary))', strokeWidth: 2, r: 3 }}
                    name="diversity"
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Parameter Evolution */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Settings className="h-5 w-5" />
              Parameter Evolution
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {parameterData.map((param, index) => (
                <div key={index} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{param.name}</span>
                    <Badge variant="secondary" className="text-xs">
                      {param.improvement}
                    </Badge>
                  </div>
                  
                  <div className="flex items-center gap-4 text-sm">
                    <div className="flex-1">
                      <div className="flex justify-between text-xs text-muted-foreground mb-1">
                        <span>Current: {param.current}</span>
                        <span>Optimal: {param.optimal}</span>
                      </div>
                      <div className="w-full bg-gray-200 rounded-full h-2 dark:bg-gray-700">
                        <div 
                          className="bg-primary h-2 rounded-full transition-all duration-300"
                          style={{ 
                            width: `${Math.min((param.optimal / Math.max(param.current, param.optimal)) * 100, 100)}%` 
                          }}
                        ></div>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Optimization Status */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Brain className="h-5 w-5" />
              Current Status
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Optimization State</span>
                <Badge variant={gepaMetrics?.isOptimizing ? "default" : "secondary"}>
                  {gepaMetrics?.isOptimizing ? "Running" : "Idle"}
                </Badge>
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Last Run</span>
                <span className="text-sm font-medium">
                  {gepaMetrics?.lastOptimization ? 
                    new Date(gepaMetrics.lastOptimization).toLocaleDateString() : 
                    'Never'
                  }
                </span>
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Population Size</span>
                <span className="text-sm font-medium">50 individuals</span>
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Mutation Rate</span>
                <span className="text-sm font-medium">10%</span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Target className="h-5 w-5" />
              Optimization Goals
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Sharpe Ratio</span>
                <span className="text-sm font-medium">30% weight</span>
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Total Return</span>
                <span className="text-sm font-medium">25% weight</span>
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Max Drawdown</span>
                <span className="text-sm font-medium">-20% weight</span>
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Risk-Adjusted Return</span>
                <span className="text-sm font-medium">35% weight</span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BarChart3 className="h-5 w-5" />
              Performance Impact
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="text-center">
                <div className="text-3xl font-bold text-green-600">
                  {gepaMetrics ? `+${gepaMetrics.improvementPercent.toFixed(1)}%` : '+0.0%'}
                </div>
                <div className="text-sm text-muted-foreground">Total Improvement</div>
              </div>
              
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Before GEPA</span>
                  <span className="font-medium">
                    {performanceMetrics ? `${performanceMetrics.sharpeRatio.toFixed(2)}` : '0.00'}
                  </span>
                </div>
                
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">After GEPA</span>
                  <span className="font-medium text-green-600">
                    {performanceMetrics && gepaMetrics ? 
                      `${(performanceMetrics.sharpeRatio * (1 + gepaMetrics.improvementPercent / 100)).toFixed(2)}` : 
                      '0.00'
                    }
                  </span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
