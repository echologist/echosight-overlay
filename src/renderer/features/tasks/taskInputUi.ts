export function readTaskInput(): string {
  return getTaskInput()?.value.trim() || '';
}

export function clearTaskInput(): void {
  const input = getTaskInput();
  if (input) {
    input.value = '';
  }
}

function getTaskInput(): HTMLInputElement | null {
  const element = document.getElementById('taskInput');
  return element instanceof HTMLInputElement ? element : null;
}
