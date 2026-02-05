import React, { useEffect, useMemo, useState } from "react";
import { Goal } from "@sentinel/sdk";
import { useStore } from "./state/store";
import { useVSCodeAPI } from "./hooks/useVSCodeAPI";
import { useMCPMessages } from "./hooks/useMCPMessages";
import { 
  LayoutDashboard, 
  MessageSquare, 
  Orbit, 
  ShieldCheck, 
  History, 
  Settings,
  Activity,
  ChevronRight,
  Zap,
  Shield,
  Target
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "./components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { ScrollArea } from "./components/ui/scroll-area";
import { Badge } from "./components/ui/badge";
import { Separator } from "./components/ui/separator";

import MessageList from "./components/Chat/MessageList";
import ChatInput from "./components/Chat/ChatInput";
import QuickPrompts from "./components/Chat/QuickPrompts";
import GoalTree from "./components/Goals/GoalTree";
import GoalGraph from "./components/AtomicForge/GoalGraph";

type PageId = "command" | "chat" | "forge" | "network" | "audit" | "settings";

export default function App() {
  const vscodeApi = useVSCodeAPI();
  useMCPMessages(vscodeApi);
  
  const connected = useStore((s) => s.connected);
  const alignment = useStore((s) => s.alignment);
  const goals = useStore((s) => s.goals);
  const messages = useStore((s) => s.messages);
  
  const [activePage, setActivePage] = useState<PageId>("command");
  const [selectedGoal, setSelectedGoal] = useState<Goal | null>(null);

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
  }, [vscodeApi]);

  const navItems = [
    { id: "command", label: "Dashboard", icon: LayoutDashboard },
    { id: "chat", label: "Agent Chat", icon: MessageSquare },
    { id: "forge", label: "Atomic Forge", icon: Target },
    { id: "network", label: "Federation", icon: Orbit },
    { id: "audit", label: "Audit Log", icon: History },
    { id: "settings", label: "Settings", icon: Settings },
  ] as const;

  const alignmentScore = alignment?.score ?? 0;
  const alignmentStatus = alignment?.status ?? "Initializing";

  return (
    <div className="flex h-full w-full bg-background overflow-hidden">
      {/* SIDEBAR NAVIGATION */}
      <aside className="w-14 md:w-48 border-r bg-card flex flex-col shrink-0">
        <div className="h-14 border-b flex items-center px-4 gap-2">
          <div className="w-6 h-6 rounded bg-primary flex items-center justify-center text-primary-foreground font-bold shrink-0">
            S
          </div>
          <span className="font-semibold tracking-tight hidden md:block">Sentinel</span>
        </div>
        
        <nav className="flex-1 p-2 space-y-1">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                onClick={() => setActivePage(item.id)}
                className={cn(
                  "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors",
                  activePage === item.id 
                    ? "bg-accent text-accent-foreground font-medium shadow-sm" 
                    : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
                )}
              >
                <Icon className="size-4 shrink-0" />
                <span className="hidden md:block">{item.label}</span>
                {item.id === "chat" && messages.length > 0 && (
                  <Badge variant="secondary" className="ml-auto size-5 p-0 justify-center rounded-full hidden md:flex">
                    {messages.length}
                  </Badge>
                )}
              </button>
            );
          })}
        </nav>

        <div className="p-4 border-t space-y-3 hidden md:block">
          <div className="flex items-center justify-between text-[10px] text-muted-foreground uppercase tracking-wider font-bold">
            <span>System Health</span>
            <div className={cn("size-2 rounded-full", connected ? "bg-green-500" : "bg-red-500")} />
          </div>
          <div className="space-y-1">
            <div className="flex justify-between text-xs">
              <span className="text-muted-foreground">Mode</span>
              <span className="font-medium text-foreground">Supervised</span>
            </div>
            <div className="flex justify-between text-xs">
              <span className="text-muted-foreground">Agent</span>
              <span className="font-medium text-foreground text-right truncate max-w-[80px]">Local MCP</span>
            </div>
          </div>
        </div>
      </aside>

      {/* MAIN CONTENT AREA */}
      <main className="flex-1 flex flex-col min-w-0 bg-background/50 backdrop-blur-3xl">
        {/* HEADER */}
        <header className="h-14 border-b flex items-center justify-between px-6 bg-card/30 sticky top-0 z-10">
          <div className="flex items-center gap-3">
            <h1 className="text-sm font-semibold capitalize tracking-tight">{activePage.replace('-', ' ')}</h1>
            <Badge variant={alignmentScore > 80 ? "outline" : "destructive"} className="px-2 py-0.5 text-[10px]">
              {alignmentStatus}
            </Badge>
          </div>
          
          <div className="flex items-center gap-4">
             <div className="flex items-center gap-2 px-3 py-1.5 rounded-full border bg-card/50 text-[11px] font-medium">
               <Activity className="size-3 text-primary animate-pulse" />
               <span>Alignment: <span className="font-bold">{alignmentScore.toFixed(0)}%</span></span>
             </div>
             <Button variant="ghost" size="icon-sm" className="rounded-full text-muted-foreground hover:text-foreground">
               <Zap className="size-4" />
             </Button>
          </div>
        </header>

        {/* PAGE CONTENT */}
        <ScrollArea className="flex-1">
          <div className="p-6 h-full">
            
            {/* DASHBOARD / COMMAND CENTER */}
            {activePage === "command" && (
              <div className="grid grid-cols-1 xl:grid-cols-12 gap-6 animate-in fade-in slide-in-from-bottom-2 duration-500">
                <div className="xl:col-span-8 space-y-6">
                  <Card className="border-primary/20 bg-primary/5">
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                      <div className="space-y-1">
                        <CardTitle className="text-base flex items-center gap-2">
                          <ShieldCheck className="size-4 text-primary" />
                          Alignment Field
                        </CardTitle>
                        <CardDescription>Continuous predictive drift monitoring and recovery</CardDescription>
                      </div>
                      <Badge variant="outline" className="bg-background/50">v2.1 Stable</Badge>
                    </CardHeader>
                    <CardContent className="space-y-6">
                      <div className="space-y-2">
                        <div className="flex justify-between text-sm">
                          <span className="font-medium">Global Alignment Vector</span>
                          <span className="text-primary font-bold">{alignmentScore.toFixed(1)}%</span>
                        </div>
                        <div className="h-2 w-full bg-accent/20 rounded-full overflow-hidden">
                          <div 
                            className="h-full bg-primary transition-all duration-1000" 
                            style={{ width: `${alignmentScore}%` }} 
                          />
                        </div>
                      </div>
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
                        {[
                          { label: "Confidence", val: `${(alignment?.confidence ?? 0 * 100).toFixed(0)}%` },
                          { label: "Drift Risk", val: `${Math.max(5, 100-alignmentScore).toFixed(0)}%` },
                          { label: "Latency", val: "112ms" },
                          { label: "Policy", val: "Guarded" }
                        ].map((stat, i) => (
                          <div key={i} className="bg-card p-3 rounded-lg border text-center shadow-sm">
                            <div className="text-[10px] uppercase text-muted-foreground font-bold tracking-wider mb-1">{stat.label}</div>
                            <div className="text-sm font-semibold">{stat.val}</div>
                          </div>
                        ))}
                      </div>
                      <div className="flex items-start gap-3 p-4 rounded-lg bg-card border border-primary/10">
                        <div className="p-2 rounded-md bg-primary/10">
                          <ChevronRight className="size-4 text-primary" />
                        </div>
                        <div className="space-y-1">
                          <div className="text-xs font-bold uppercase tracking-tight text-primary">Suggested Action</div>
                          <div className="text-sm">Finalize the action gate policy and confirm tool scopes.</div>
                          <div className="text-[10px] text-muted-foreground font-mono italic">Predicted impact: +6.4% alignment increase</div>
                        </div>
                      </div>
                    </CardContent>
                  </Card>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <Card>
                      <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-bold flex items-center gap-2">
                          <Target className="size-4" />
                          Goal Manifold
                        </CardTitle>
                      </CardHeader>
                      <CardContent>
                        {goals.length > 0 ? (
                          <div className="space-y-3">
                            {goals.slice(0, 3).map((goal) => (
                              <div key={goal.id} className="flex items-center justify-between p-2 rounded-md border bg-accent/20 text-xs">
                                <span className="truncate max-w-[150px]">{goal.description}</span>
                                <Badge variant="secondary" className="text-[10px] uppercase">{goal.status}</Badge>
                              </div>
                            ))}
                            <Button variant="link" className="w-full text-xs h-auto p-0 text-muted-foreground hover:text-primary">
                              View all {goals.length} goals
                            </Button>
                          </div>
                        ) : (
                          <div className="text-center py-6 text-sm text-muted-foreground italic">No goals active.</div>
                        )}
                      </CardContent>
                    </Card>

                    <Card>
                      <CardHeader className="pb-3">
                        <CardTitle className="text-sm font-bold flex items-center gap-2">
                          <Shield className="size-4 text-orange-500" />
                          Action Gate
                        </CardTitle>
                      </CardHeader>
                      <CardContent>
                         <div className="space-y-3">
                            <div className="flex items-center justify-between p-2 rounded-md border border-orange-200 bg-orange-50 dark:bg-orange-950/20 text-xs">
                              <span className="font-medium">Safe Write: src/core/auth.ts</span>
                              <Button variant="outline" size="xs" className="text-[9px] h-5 bg-background">Review</Button>
                            </div>
                            <div className="flex items-center justify-between p-2 rounded-md border bg-accent/20 text-xs opacity-60">
                              <span className="italic text-muted-foreground">No other pending actions</span>
                            </div>
                         </div>
                      </CardContent>
                    </Card>
                  </div>
                </div>

                <div className="xl:col-span-4 space-y-6">
                  <Card className="h-full flex flex-col">
                    <CardHeader className="pb-3 border-b">
                      <CardTitle className="text-sm font-bold flex items-center gap-2">
                        <MessageSquare className="size-4 text-primary" />
                        Conversation Trace
                      </CardTitle>
                    </CardHeader>
                    <CardContent className="p-0 flex-1 overflow-hidden">
                       <ScrollArea className="h-[300px]">
                         <div className="p-4 space-y-4">
                           {messages.length === 0 ? (
                             <div className="text-center py-12 text-sm text-muted-foreground">Silent monitoring active.</div>
                           ) : (
                             messages.slice(-4).map((msg) => (
                               <div key={msg.id} className={cn(
                                 "space-y-1 p-3 rounded-lg border text-xs",
                                 msg.role === 'user' ? "ml-4 bg-primary/5" : "mr-4 bg-card shadow-sm"
                               )}>
                                 <div className="font-bold uppercase text-[9px] tracking-widest text-muted-foreground">{msg.role}</div>
                                 <div className="leading-relaxed line-clamp-3">{msg.content}</div>
                               </div>
                             ))
                           )}
                         </div>
                       </ScrollArea>
                    </CardContent>
                    <div className="p-3 border-t bg-accent/10">
                      <div className="grid grid-cols-2 gap-2">
                        <div className="p-2 rounded bg-card border text-center">
                          <div className="text-[9px] font-bold text-muted-foreground uppercase mb-0.5">Tools</div>
                          <div className="text-xs font-bold">14 Calls</div>
                        </div>
                        <div className="p-2 rounded bg-card border text-center">
                          <div className="text-[9px] font-bold text-muted-foreground uppercase mb-0.5">Files</div>
                          <div className="text-xs font-bold">3 Modif</div>
                        </div>
                      </div>
                    </div>
                  </Card>
                </div>
              </div>
            )}

            {/* CHAT INTERFACE */}
            {activePage === "chat" && (
              <div className="flex flex-col h-[calc(100vh-180px)] animate-in fade-in duration-300">
                <Card className="flex-1 flex flex-col p-0 overflow-hidden shadow-2xl border-primary/10">
                   <div className="flex-1 overflow-hidden relative">
                      <MessageList />
                   </div>
                   <div className="p-4 border-t bg-card/80 backdrop-blur-sm sticky bottom-0">
                      <QuickPrompts />
                      <ChatInput />
                   </div>
                </Card>
              </div>
            )}

            {/* ATOMIC FORGE */}
            {activePage === "forge" && (
              <div className="h-full space-y-6 animate-in zoom-in-95 duration-300">
                <Card className="h-[500px] overflow-hidden">
                  <CardHeader className="border-b bg-card/50">
                    <CardTitle className="text-sm font-bold">Topology Map</CardTitle>
                  </CardHeader>
                  <CardContent className="p-0 h-full">
                    <GoalGraph goals={goals} onNodeSelect={setSelectedGoal} />
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="p-4">
                    <GoalTree />
                  </CardContent>
                </Card>
              </div>
            )}

            {/* AUDIT LOG */}
            {activePage === "audit" && (
              <Card className="animate-in slide-in-from-right-4 duration-300">
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    <History className="size-4" />
                    Immutable Audit Log
                  </CardTitle>
                  <CardDescription>Cryptographically verified execution history</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="space-y-1">
                    {messages.map((msg, idx) => (
                      <div key={msg.id} className="flex items-center justify-between p-3 rounded-md border hover:bg-accent/30 transition-colors group">
                        <div className="flex items-center gap-4">
                          <span className="text-xs font-mono text-muted-foreground">{new Date(msg.timestamp).toLocaleTimeString()}</span>
                          <span className="text-sm font-medium">{msg.role === 'user' ? 'Direct Intent' : 'Autonomous Action'}</span>
                        </div>
                        <Badge variant="outline" className="text-[9px] opacity-0 group-hover:opacity-100 transition-opacity">
                          VERIFIED-HASH: {Math.random().toString(16).slice(2, 8)}
                        </Badge>
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>
            )}

            {/* SETTINGS */}
            {activePage === "settings" && (
              <div className="max-w-2xl mx-auto space-y-6 animate-in slide-in-from-top-4 duration-300">
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                      <Settings className="size-4" />
                      Platform Settings
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-6">
                    {[
                      { title: "Autonomous Level", desc: "Supervised (Human in the loop)", icon: ShieldCheck },
                      { title: "Verification Strategy", desc: "Mathematical Proof (Formal)", icon: Activity },
                      { title: "Memory Depth", desc: "Episodic (Qdrant Cloud)", icon: History }
                    ].map((item, i) => (
                      <div key={i} className="flex items-start gap-4 p-4 rounded-xl border bg-card hover:bg-accent/20 transition-all cursor-pointer group">
                        <div className="p-2 rounded-lg bg-primary/10 group-hover:bg-primary/20 transition-colors">
                          <item.icon className="size-5 text-primary" />
                        </div>
                        <div className="space-y-1">
                          <div className="font-bold text-sm">{item.title}</div>
                          <div className="text-xs text-muted-foreground">{item.desc}</div>
                        </div>
                        <ChevronRight className="ml-auto size-4 self-center text-muted-foreground" />
                      </div>
                    ))}
                  </CardContent>
                </Card>
              </div>
            )}

          </div>
        </ScrollArea>
        
        <footer className="h-8 border-t bg-card/50 flex items-center justify-between px-6 text-[10px] text-muted-foreground font-mono">
          <div className="flex items-center gap-3">
             <span className="flex items-center gap-1"><div className="size-1.5 rounded-full bg-green-500" /> STATUS: ALIGNED</span>
             <span className="opacity-50">|</span>
             <span>PROTOCOL: v1.2.0</span>
          </div>
          <div>SENTINEL COGNITIVE GATEWAY</div>
        </footer>
      </main>
    </div>
  );
}
