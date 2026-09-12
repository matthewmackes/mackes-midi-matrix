(() => {
  const root = document.querySelector('#studio-controller');
  if (!root) return;
  const groups = [
    { key: 'knob', label: 'KNOBS', count: 24, columns: 8, led: true },
    { key: 'channel-button', label: 'CHANNEL BUTTONS', count: 16, columns: 8, led: true },
    { key: 'fader', label: 'FADERS', count: 8, columns: 8, led: false },
    { key: 'utility', label: 'UTILITY CONTROLS', count: 8, columns: 8, led: true },
  ];
  const controls = [];
  const utilityNames = ['Device', 'Mute', 'Solo', 'Record', 'Up', 'Down', 'Left', 'Right'];
  root.replaceChildren();
  groups.forEach(group => {
    const section = document.createElement('section');
    section.className = `control-group control-group-${group.key}`;
    section.style.setProperty('--control-columns', group.columns);
    const heading = document.createElement('h3');
    heading.className = 'control-group-label';
    heading.textContent = group.label;
    section.append(heading);
    const grid = document.createElement('div');
    grid.className = 'control-grid';
    for (let index = 0; index < group.count; index += 1) {
      const row = Math.floor(index / 8) + 1;
      const column = (index % 8) + 1;
      const id = group.key === 'knob' ? `knob-r${row}-c${column}`
        : group.key === 'channel-button' ? `button-r${row}-c${column}`
          : `${group.key}-${index + 1}`;
      const button = document.createElement('button');
      button.type = 'button';
      button.className = `physical-control physical-control-${group.key}`;
      button.dataset.controlId = id;
      button.dataset.controlType = group.key;
      button.setAttribute('aria-label', `${group.label.toLowerCase()} ${index + 1}; ${group.led ? 'LED capable' : 'no LED'}; unassigned`);
      const visual = document.createElement('span');
      visual.className = `control-visual ${group.led ? 'has-led' : 'no-led'}`;
      visual.setAttribute('aria-hidden', 'true');
      if (group.key === 'fader') visual.append(document.createElement('span'));
      const label = document.createElement('span');
      label.className = 'physical-control-label';
      label.textContent = group.key === 'channel-button' ? `B${row}.${column}` : group.key === 'utility' ? utilityNames[index] : `${group.key === 'fader' ? 'F' : 'K'}${String(index + 1).padStart(2, '0')}`;
      const assignment = document.createElement('span');
      assignment.className = 'control-assignment';
      assignment.textContent = 'Unassigned';
      const value = document.createElement('span');
      value.className = 'control-value';
      value.textContent = 'Value unknown';
      const sync = document.createElement('span');
      sync.className = 'control-sync';
      sync.textContent = 'Sync pending';
      button.append(visual, label, assignment, value, sync);
      button.addEventListener('click', () => {
        controls.forEach(item => item.classList.remove('is-selected'));
        button.classList.add('is-selected');
        const detail = { id, type: group.key, label: group.key === 'utility' ? utilityNames[index] : `${group.label.toLowerCase()} ${index + 1}`, led: group.led };
        window.MackesStudioState?.publish({ selectedControl: detail });
        root.dispatchEvent(new CustomEvent('studio-control-selected', { bubbles: true, detail }));
      });
      controls.push(button);
      grid.append(button);
    }
    section.append(grid);
    root.append(section);
  });
  const note = document.createElement('p');
  note.className = 'surface-note';
  note.textContent = 'Select a control to inspect its assignment.';
  root.append(note);
  const restored = window.MackesStudioState?.read().selectedControl;
  if (restored?.id) controls.find(item => item.dataset.controlId === restored.id)?.click();
})();
