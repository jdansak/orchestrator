import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useState, type FormEvent } from 'react';
import { addRecord, deleteRecord, type DnsZone } from '../api/endpoints';

/** The DNS zone editor (G8.6). */
export default function DnsZoneEditor({ tenant, zones }: { tenant: string; zones: DnsZone[] }) {
  const queryClient = useQueryClient();
  const [zone, setZone] = useState(zones[0]?.zone ?? '');
  const [name, setName] = useState('');

  const add = useMutation({
    mutationFn: ({ zone: targetZone, recordName }: { zone: string; recordName: string }) =>
      addRecord(tenant, targetZone, { name: recordName, type: 'A', value: '10.0.0.100' }),
    onSuccess: () => {
      setName('');
      queryClient.invalidateQueries({ queryKey: ['zones', tenant] });
    },
  });

  const remove = useMutation({
    mutationFn: ({ zone: targetZone, recordName }: { zone: string; recordName: string }) =>
      deleteRecord(tenant, targetZone, recordName),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['zones', tenant] }),
  });

  function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (zone === '' || name === '') return;
    add.mutate({ zone, recordName: name });
  }

  return (
    <section aria-label="DNS zones">
      <form onSubmit={onSubmit}>
        <label htmlFor="dns-zone">Zone</label>
        <select id="dns-zone" value={zone} onChange={(event) => setZone(event.target.value)}>
          {zones.map((candidate) => (
            <option key={candidate.zone} value={candidate.zone}>
              {candidate.zone}
            </option>
          ))}
        </select>
        <label htmlFor="dns-record-name">Record name</label>
        <input
          id="dns-record-name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
        />
        <button type="submit" disabled={add.isPending}>
          Add record
        </button>
      </form>

      {add.isError ? <p role="alert">The record could not be added.</p> : null}

      {zones.map((candidate) => (
        <div key={candidate.zone}>
          <h2>{candidate.zone}</h2>
          <ul>
            {candidate.records.map((record) => (
              <li key={record.name}>
                {record.name} {record.type} {record.value ?? record.target ?? ''}
                <button
                  type="button"
                  onClick={() =>
                    remove.mutate({ zone: candidate.zone, recordName: record.name })
                  }
                >
                  Delete
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
