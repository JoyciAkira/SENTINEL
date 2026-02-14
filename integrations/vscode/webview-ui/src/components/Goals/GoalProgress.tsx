import React from "react";

interface Props {
  completed: number;
  total: number;
}

export default function GoalProgress({ completed, total }: Props) {
  if (total === 0) return null;
  const pct = Math.round((completed / total) * 100);

  return (
    <span className="goal-progress">
      ({completed}/{total} done)
    </span>
  );
}
