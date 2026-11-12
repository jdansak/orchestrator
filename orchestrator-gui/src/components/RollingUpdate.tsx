import { useMutation, useQueryClient } from '@tanstack/react-query';
import { advanceRollout, rollbackRollout, type RolloutState } from '../api/endpoints';

/** The rolling-update panel (G8.5). */
export default function RollingUpdate({
  tenant,
  rollout,
}: {
  tenant: string;
  rollout: RolloutState | null;
}) {
  const queryClient = useQueryClient();

  const advance = useMutation({
    mutationFn: () => advanceRollout(tenant),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['rollout', tenant] }),
  });

  const rollback = useMutation({
    mutationFn: () => rollbackRollout(tenant),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['rollout', tenant] }),
  });

  if (!rollout) return <p>No rollout is configured for this tenant.</p>;

  const percent = rollout.green_percent;

  return (
    <section aria-label="Rolling update">
      <p>
        State: <strong>{rollout.state}</strong> — step {rollout.current_step} of{' '}
        {rollout.steps.length - 1}
      </p>
      <div
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label="Traffic on green"
      >
        <span style={{ width: `${percent}%` }} />
      </div>
      <button type="button" onClick={() => advance.mutate()} disabled={advance.isPending}>
        Next step
      </button>
      <button type="button" onClick={() => rollback.mutate()} disabled={rollback.isPending}>
        Roll back
      </button>
      {rollout.rollback ? <p role="alert">Rolled back: {rollout.rollback.reason}</p> : null}
    </section>
  );
}
