/**
 * QualityDashboard Component
 *
 * Displays quality evaluation results with dimension scores, gate status,
 * and overall verdict for the generated artifacts.
 */

import React, { useMemo } from 'react';
import { useStore } from '../../state/store';
import {
  Shield,
  ShieldCheck,
  ShieldAlert,
  Gauge,
  TrendingUp,
  TrendingDown,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Activity,
  Clock,
  Cpu,
  FileCheck,
  Layers,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { Progress } from '../ui/progress';
import { ScrollArea } from '../ui/scroll-area';
import { cn } from '../../lib/utils';

interface QualityDashboardProps {
  className?: string;
}

const METRIC_INFO = {
  Correctness: {
    label: 'Correctness',
    description: 'Does the artifact meet functional requirements?',
    color: 'text-blue-500',
    bgColor: 'bg-blue-500/10',
  },
  Reliability: {
    label: 'Reliability',
    description: 'Does it work consistently under expected conditions?',
    color: 'text-green-500',
    bgColor: 'bg-green-500/10',
  },
  Maintainability: {
    label: 'Maintainability',
    description: 'Is it easy to understand and modify?',
    color: 'text-purple-500',
    bgColor: 'bg-purple-500/10',
  },
  Security: {
    label: 'Security',
    description: 'Does it follow security best practices?',
    color: 'text-red-500',
    bgColor: 'bg-red-500/10',
  },
  UXDevEx: {
    label: 'UX/DevEx',
    description: 'Is the user/developer experience good?',
    color: 'text-amber-500',
    bgColor: 'bg-amber-500/10',
  },
} as const;

function getScoreColor(value: number, threshold: number): string {
  if (value >= threshold) return 'text-green-500';
  if (value >= threshold * 0.9) return 'text-yellow-500';
  return 'text-red-500';
}

function getScoreBgColor(value: number, threshold: number): string {
  if (value >= threshold) return 'bg-green-500/10';
  if (value >= threshold * 0.9) return 'bg-yellow-500/10';
  return 'bg-red-500/10';
}

function shortId(id: string): string {
  return id.replace(/-/g, '').slice(0, 8).toUpperCase();
}

export function QualityDashboard({ className }: QualityDashboardProps) {
  const qualityDashboard = useStore((s) => s.qualityDashboard);

  if (!qualityDashboard || !qualityDashboard.latest_report) {
    return (
      <Card className={cn('sentinel-quality-dashboard', className)}>
        <CardContent className="flex items-center justify-center py-12">
          <div className="text-center">
            <Shield className="h-12 w-12 mx-auto mb-4 text-sentinel-muted-foreground opacity-50" />
            <p className="text-sentinel-muted-foreground">No quality data available</p>
            <p className="text-xs text-sentinel-muted-foreground mt-2">
              Quality evaluation data appears after artifact generation
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  const report = qualityDashboard.latest_report;
  const hasFailures = report.scores.some((s) => s.result === 'Fail');
  const overallPass = report.overall === 'Pass';

  const weightedScore = useMemo(() => {
    const weights = {
      Correctness: 0.30,
      Reliability: 0.25,
      Maintainability: 0.20,
      Security: 0.15,
      UXDevEx: 0.10,
    };
    const total = report.scores.reduce((sum, score) => {
      const weight = weights[score.metric as keyof typeof weights] || 0;
      return sum + score.value * weight;
    }, 0);
    return total;
  }, [report.scores]);

  return (
    <Card className={cn('sentinel-quality-dashboard', className)}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {overallPass ? (
              <ShieldCheck className="h-5 w-5 text-green-500" />
            ) : (
              <ShieldAlert className="h-5 w-5 text-red-500" />
            )}
            <CardTitle>Quality Dashboard</CardTitle>
            <Badge variant={overallPass ? 'default' : 'destructive'}>
              {report.overall}
            </Badge>
          </div>
          <div className="flex items-center gap-2 text-xs text-sentinel-muted-foreground">
            <Activity className="h-3 w-3" />
            <span>v{qualityDashboard.rubric_version}</span>
            <span>•</span>
            <span>{qualityDashboard.total_evaluations} evals</span>
          </div>
        </div>
        <CardDescription>
          <div className="flex items-center justify-between">
            <span>
              Run: <span className="font-mono ml-1">{shortId(report.run_id)}</span>
            </span>
            <span>
              Module: <span className="font-mono ml-1">{shortId(report.module_id)}</span>
            </span>
          </div>
        </CardDescription>
      </CardHeader>

      <CardContent className="p-0">
        <ScrollArea className="h-[500px]">
          <div className="p-4 space-y-4">
            {/* Overall Score Gauge */}
            <div className="p-4 rounded-lg bg-sentinel-accent/5 border border-sentinel-accent/20">
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-semibold flex items-center gap-2">
                  <Gauge className="h-4 w-4" />
                  Overall Quality Score
                </h3>
                <div className="flex items-center gap-1">
                  {weightedScore >= 85 ? (
                    <CheckCircle2 className="h-4 w-4 text-green-500" />
                  ) : weightedScore >= 70 ? (
                    <AlertCircle className="h-4 w-4 text-yellow-500" />
                  ) : (
                    <XCircle className="h-4 w-4 text-red-500" />
                  )}
                  <span className={cn('text-xl font-bold', getScoreColor(weightedScore, 85))}>
                    {weightedScore.toFixed(1)}
                  </span>
                  <span className="text-sentinel-muted-foreground text-sm">/100</span>
                </div>
              </div>
              <Progress
                value={weightedScore}
                className="h-2"
                indicatorClassName={cn(
                  weightedScore >= 85
                    ? 'bg-green-500'
                    : weightedScore >= 70
                    ? 'bg-yellow-500'
                    : 'bg-red-500'
                )}
              />
            </div>

            {/* Dimension Scores */}
            <div>
              <h4 className="text-xs font-semibold uppercase tracking-wide text-sentinel-muted-foreground mb-3 flex items-center gap-1">
                <Layers className="h-3 w-3" />
                Dimension Scores
              </h4>
              <div className="grid grid-cols-1 gap-3">
                {report.scores.map((score) => {
                  const info = METRIC_INFO[score.metric];
                  const scoreColor = getScoreColor(score.value, score.threshold);
                  const bgColor = getScoreBgColor(score.value, score.threshold);
                  const scorePercent = Math.min(100, (score.value / score.threshold) * 100);

                  return (
                    <div
                      key={score.metric}
                      className="p-3 rounded-lg border border-sentinel-muted-foreground/20"
                    >
                      <div className="flex items-start justify-between mb-2">
                        <div className="flex items-center gap-2">
                          <div className={cn('p-1.5 rounded', info.bgColor)}>
                            {score.result === 'Pass' ? (
                              <CheckCircle2 className={cn('h-3.5 w-3.5', info.color)} />
                            ) : (
                              <XCircle className="h-3.5 w-3.5 text-sentinel-muted-foreground" />
                            )}
                          </div>
                          <div>
                            <div className="text-sm font-medium">{info.label}</div>
                            <div className="text-xs text-sentinel-muted-foreground">
                              {info.description}
                            </div>
                          </div>
                        </div>
                        <Badge
                          variant={score.gate === 'Hard' ? 'destructive' : 'secondary'}
                          className="text-xs"
                        >
                          {score.gate}
                        </Badge>
                      </div>

                      <div className="flex items-center gap-3">
                        <div className="flex-1">
                          <div className="flex items-center justify-between text-xs text-sentinel-muted-foreground mb-1">
                            <span>Score</span>
                            <span>
                              {score.value.toFixed(1)} / {score.threshold}
                            </span>
                          </div>
                          <Progress
                            value={scorePercent}
                            className="h-1.5"
                            indicatorClassName={cn(
                              scorePercent >= 100
                                ? 'bg-green-500'
                                : scorePercent >= 90
                                ? 'bg-yellow-500'
                                : 'bg-red-500'
                            )}
                          />
                        </div>
                        <div
                          className={cn(
                            'text-lg font-bold font-mono w-12 text-right',
                            bgColor
                          )}
                        >
                          {score.value.toFixed(0)}
                        </div>
                      </div>

                      <div className="mt-2 pt-2 border-t border-sentinel-muted-foreground/10 flex items-center justify-between text-xs">
                        <span className={scoreColor}>
                          {score.result === 'Pass' ? 'Passing' : 'Failing'} by{' '}
                          {score.result === 'Pass'
                            ? `+${(score.value - score.threshold).toFixed(1)}`
                            : `${(score.value - score.threshold).toFixed(1)}`}
                        </span>
                        {score.gate === 'Hard' && score.result === 'Fail' && (
                          <span className="text-red-500 font-medium">Blocking</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Evaluation Metadata */}
            <div className="p-3 rounded-lg bg-sentinel-muted-foreground/5 border border-sentinel-muted-foreground/10">
              <h4 className="text-xs font-semibold uppercase tracking-wide text-sentinel-muted-foreground mb-2 flex items-center gap-1">
                <Cpu className="h-3 w-3" />
                Evaluation Metadata
              </h4>
              <div className="grid grid-cols-2 gap-2 text-xs">
                <div className="flex items-center gap-1">
                  <FileCheck className="h-3 w-3 text-sentinel-muted-foreground" />
                  <span className="text-sentinel-muted-foreground">Provider:</span>
                  <span className="font-medium">{report.metadata.llm_provider}</span>
                </div>
                <div className="flex items-center gap-1">
                  <Cpu className="h-3 w-3 text-sentinel-muted-foreground" />
                  <span className="text-sentinel-muted-foreground">Model:</span>
                  <span className="font-medium">{report.metadata.model}</span>
                </div>
                <div className="flex items-center gap-1">
                  <Clock className="h-3 w-3 text-sentinel-muted-foreground" />
                  <span className="text-sentinel-muted-foreground">Duration:</span>
                  <span className="font-medium">
                    {report.metadata.evaluation_duration_ms}ms
                  </span>
                </div>
                <div className="flex items-center gap-1">
                  <Activity className="h-3 w-3 text-sentinel-muted-foreground" />
                  <span className="text-sentinel-muted-foreground">Iteration:</span>
                  <span className="font-medium">{report.metadata.iteration}</span>
                </div>
              </div>
            </div>

            {/* Linked Artifacts */}
            {report.linked_artifact_ids.length > 0 && (
              <div>
                <h4 className="text-xs font-semibold uppercase tracking-wide text-sentinel-muted-foreground mb-2 flex items-center gap-1">
                  <FileCheck className="h-3 w-3" />
                  Linked Artifacts ({report.linked_artifact_ids.length})
                </h4>
                <div className="flex flex-wrap gap-1">
                  {report.linked_artifact_ids.map((id) => (
                    <Badge key={id} variant="outline" className="font-mono text-xs">
                      {shortId(id)}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
