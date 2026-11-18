// ==============================================================================
// Stage Manager for Windows — Website Interactive Scripts
// ==============================================================================

document.addEventListener('DOMContentLoaded', () => {
  initThemeToggle();
  initSimulator();
  initCopyButtons();
});

// 1. Dark / Light Mode Toggle
function initThemeToggle() {
  const themeBtn = document.getElementById('theme-toggle');
  if (!themeBtn) return;

  const currentTheme = localStorage.getItem('sm_theme') || 'dark';
  document.documentElement.setAttribute('data-theme', currentTheme);
  updateThemeIcon(themeBtn, currentTheme);

  themeBtn.addEventListener('click', () => {
    const nextTheme = document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', nextTheme);
    localStorage.setItem('sm_theme', nextTheme);
    updateThemeIcon(themeBtn, nextTheme);
  });
}

function updateThemeIcon(btn, theme) {
  btn.innerHTML = theme === 'dark' ? '☀️' : '🌙';
  btn.title = `Switch to ${theme === 'dark' ? 'Light' : 'Dark'} Mode`;
}

// 2. Interactive Stage Manager Simulator
function initSimulator() {
  const triggerZone = document.querySelector('.trigger-zone');
  const simSidebar = document.querySelector('.sim-sidebar');
  const stageCards = document.querySelectorAll('.stage-card');
  const mockTitle = document.querySelector('.mock-window-title');
  const canvas = document.querySelector('.screen-canvas');

  if (!triggerZone || !simSidebar) return;

  // Reveal on mouseenter over trigger zone or clicking it
  triggerZone.addEventListener('mouseenter', () => {
    simSidebar.classList.add('revealed');
    canvas.classList.add('sidebar-visible');
  });

  // Keep open when hovering sidebar
  simSidebar.addEventListener('mouseenter', () => {
    simSidebar.classList.add('revealed');
  });

  // Retract when leaving sidebar back to workspace
  simSidebar.addEventListener('mouseleave', () => {
    simSidebar.classList.remove('revealed');
    canvas.classList.remove('sidebar-visible');
  });

  // Stage Switch on Click
  const stageTitles = {
    '1': 'Google Chrome — Rust Win32 DWM API Documentation',
    '2': 'Visual Studio Code — Stage Manager in Rust (main.rs)',
    '3': 'Windows Terminal — cargo test & PowerShell',
    '4': 'File Explorer — D:\\Projects\\Stage manager'
  };

  stageCards.forEach(card => {
    card.addEventListener('click', (e) => {
      // Don't trigger parent if fanned card was clicked
      if (e.target.closest('.fanned-card')) {
        const winName = e.target.closest('.fanned-card').dataset.window || 'Window 2';
        if (mockTitle) mockTitle.textContent = `Visual Studio Code — [${winName}]`;
        return;
      }

      stageCards.forEach(c => c.classList.remove('active'));
      card.classList.add('active');

      const stageId = card.dataset.stage;
      if (mockTitle && stageTitles[stageId]) {
        mockTitle.textContent = stageTitles[stageId];
      }

      // Smoothly hide sidebar after picking stage
      setTimeout(() => {
        simSidebar.classList.remove('revealed');
        canvas.classList.remove('sidebar-visible');
      }, 300);
    });
  });

  // Simulator Toggle Buttons (Manual Trigger Mode / Dock Side)
  const btnToggle = document.getElementById('sim-toggle-sidebar');
  if (btnToggle) {
    btnToggle.addEventListener('click', () => {
      simSidebar.classList.toggle('revealed');
      canvas.classList.toggle('sidebar-visible');
    });
  }
}

// 3. Copy-to-Clipboard Functionality
function initCopyButtons() {
  const copyBtns = document.querySelectorAll('.copy-btn');
  copyBtns.forEach(btn => {
    btn.addEventListener('click', async () => {
      const targetSelector = btn.getAttribute('data-copy-target');
      const textToCopy = targetSelector ? document.querySelector(targetSelector)?.textContent?.trim() : btn.getAttribute('data-copy-text');

      if (!textToCopy) return;

      try {
        await navigator.clipboard.writeText(textToCopy);
        const originalText = btn.textContent;
        btn.textContent = 'Copied! ✓';
        btn.style.color = '#34d399';
        btn.style.borderColor = '#34d399';

        setTimeout(() => {
          btn.textContent = originalText;
          btn.style.color = '';
          btn.style.borderColor = '';
        }, 2000);
      } catch (err) {
        console.error('Failed to copy:', err);
      }
    });
  });
}
