import React from "react";
import type { AppSpecPayload } from "../../state/types";
import { Badge } from "../ui/badge";
import { ScrollArea } from "../ui/scroll-area";
import {
  Database,
  LayoutPanelTop,
  Plug,
  ShieldCheck,
  TestTube2,
  Zap,
} from "lucide-react";

function formatLabel(value: string): string {
  return value
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

export default function AppSpecPreview({ appSpec }: { appSpec: AppSpecPayload }) {
  return (
    <section className="sentinel-preview">
      <header className="sentinel-preview__header">
        <div>
          <p className="sentinel-preview__eyebrow">Live Preview</p>
          <h3>{appSpec.app.name}</h3>
          <p>{appSpec.app.summary}</p>
        </div>
        <Badge variant="outline">AppSpec v{appSpec.version}</Badge>
      </header>

      <ScrollArea className="sentinel-preview__body">
        <article className="sentinel-preview__section">
          <div className="sentinel-preview__title">
            <Database className="size-3.5" />
            Data Model
          </div>
          {appSpec.dataModel.entities.map((entity) => (
            <div key={entity.name} className="sentinel-preview__entity">
              <strong>{formatLabel(entity.name)}</strong>
              <div className="sentinel-preview__chips">
                {entity.fields.slice(0, 6).map((field) => (
                  <span key={`${entity.name}-${field.name}`}>
                    {field.name}:{field.type}
                    {field.required ? "*" : ""}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </article>

        <article className="sentinel-preview__section">
          <div className="sentinel-preview__title">
            <LayoutPanelTop className="size-3.5" />
            Views
          </div>
          <ul>
            {appSpec.views.map((view) => (
              <li key={view.id}>
                {formatLabel(view.title)}
                {view.entity ? ` (${view.entity})` : ""}
              </li>
            ))}
          </ul>
        </article>

        <article className="sentinel-preview__section">
          <div className="sentinel-preview__title">
            <Zap className="size-3.5" />
            Actions
          </div>
          <ul>
            {appSpec.actions.map((action) => (
              <li key={action.id}>
                {formatLabel(action.title)}
                {action.requiresApproval ? " [approval]" : ""}
              </li>
            ))}
          </ul>
        </article>

        <article className="sentinel-preview__section">
          <div className="sentinel-preview__title">
            <ShieldCheck className="size-3.5" />
            Policies
          </div>
          <ul>
            {appSpec.policies.map((policy) => (
              <li key={policy.id}>
                [{policy.level}] {policy.rule}
              </li>
            ))}
          </ul>
        </article>

        <article className="sentinel-preview__section">
          <div className="sentinel-preview__title">
            <Plug className="size-3.5" />
            Integrations
          </div>
          {appSpec.integrations.length === 0 ? (
            <p className="sentinel-empty">No integrations detected yet.</p>
          ) : (
            <ul>
              {appSpec.integrations.map((integration) => (
                <li key={integration.id}>
                  {integration.provider}: {integration.purpose}
                </li>
              ))}
            </ul>
          )}
        </article>

        <article className="sentinel-preview__section">
          <div className="sentinel-preview__title">
            <TestTube2 className="size-3.5" />
            Tests
          </div>
          <ul>
            {appSpec.tests.map((test) => (
              <li key={test.id}>
                [{test.type}] {test.description}
              </li>
            ))}
          </ul>
        </article>
      </ScrollArea>

      <footer className="sentinel-preview__footer">
        <span>
          Source: {appSpec.meta.source}
          {appSpec.meta.validation ? ` • ${appSpec.meta.validation.status}` : ""}
        </span>
        <span>Confidence: {(appSpec.meta.confidence * 100).toFixed(0)}%</span>
      </footer>
      {appSpec.meta.validation?.issues?.length ? (
        <div className="sentinel-preview__issues">
          {appSpec.meta.validation.issues.slice(0, 3).map((issue) => (
            <p key={issue}>{issue}</p>
          ))}
        </div>
      ) : null}
    </section>
  );
}
