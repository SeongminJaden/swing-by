import { ipcMain, shell, BrowserWindow } from 'electron';
import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import os from 'os';
import http from 'http';

// Supabase integration (optional - falls back to local storage)
let getSupabase: (() => any) | null = null;
try {
  const mod = require('./supabase');
  getSupabase = mod.getSupabase;
} catch { /* supabase module not available */ }

// Types
interface Plan {
  id: string;
  name: string;
  price: number;
  interval: 'month' | 'custom';
  features: string[];
  stripePriceId?: string;
}

interface Subscription {
  plan: string;
  status: 'active' | 'canceled' | 'past_due' | 'trialing' | 'none';
  expiresAt: string;
  cancelAt?: string;
  subscribedAt: string;
  stripeSubscriptionId?: string;
}

interface Invoice {
  id: string;
  amount: number;
  date: string;
  status: 'paid' | 'pending' | 'failed';
  plan: string;
}

interface PaymentStore {
  stripeKey: string | null;
  stripeKeyEncrypted: boolean;
  customerId: string | null;
  subscriptionId: string | null;
  subscription: Subscription | null;
  invoices: Invoice[];
}

// Constants
const DATA_DIR = path.join(os.homedir(), '.videplace');
const PAYMENT_FILE = path.join(DATA_DIR, 'payment.json');

const LOCAL_PLANS: Plan[] = [
  {
    id: 'free',
    name: 'Free',
    price: 0,
    interval: 'month',
    features: [
      'Basic code editing',
      'Single workspace',
      'Community support',
      'Basic AI chat (limited)',
    ],
  },
  {
    id: 'pro',
    name: 'Pro',
    price: 12,
    interval: 'month',
    features: [
      'Unlimited workspaces',
      'Advanced AI code generation',
      'Priority support',
      'Security scanning',
      'Deploy to Vercel/Netlify',
      'Git integration',
      'Real-time monitoring',
    ],
  },
  {
    id: 'team',
    name: 'Team',
    price: 29,
    interval: 'month',
    features: [
      'Everything in Pro',
      'Team collaboration',
      'Shared workspaces',
      'Role-based access control',
      'Audit logs',
      'Custom integrations',
      'Per-seat pricing',
    ],
  },
  {
    id: 'enterprise',
    name: 'Enterprise',
    price: -1,
    interval: 'custom',
    features: [
      'Everything in Team',
      'Dedicated support',
      'Custom AI model training',
      'On-premise deployment',
      'SLA guarantees',
      'SSO/SAML integration',
      'Unlimited seats',
    ],
  },
];

// Encryption helpers using machine-specific key
function getMachineKey(): Buffer {
  const machineId = `${os.hostname()}-${os.userInfo().username}-videplace-payment`;
  return crypto.createHash('sha256').update(machineId).digest();
}

function encryptString(plaintext: string): string {
  const key = getMachineKey();
  const iv = crypto.randomBytes(16);
  const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
  let encrypted = cipher.update(plaintext, 'utf8', 'hex');
  encrypted += cipher.final('hex');
  return iv.toString('hex') + ':' + encrypted;
}

function decryptString(encrypted: string): string {
  const key = getMachineKey();
  const [ivHex, data] = encrypted.split(':');
  if (!ivHex || !data) throw new Error('Invalid encrypted format');
  const iv = Buffer.from(ivHex, 'hex');
  const decipher = crypto.createDecipheriv('aes-256-cbc', key, iv);
  let decrypted = decipher.update(data, 'hex', 'utf8');
  decrypted += decipher.final('utf8');
  return decrypted;
}

// Helpers
function ensureDataDir(): void {
  if (!fs.existsSync(DATA_DIR)) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
  }
}

function readPaymentStore(): PaymentStore {
  ensureDataDir();
  if (!fs.existsSync(PAYMENT_FILE)) {
    const empty: PaymentStore = {
      stripeKey: null,
      stripeKeyEncrypted: false,
      customerId: null,
      subscriptionId: null,
      subscription: null,
      invoices: [],
    };
    fs.writeFileSync(PAYMENT_FILE, JSON.stringify(empty, null, 2), 'utf-8');
    return empty;
  }
  try {
    const raw = fs.readFileSync(PAYMENT_FILE, 'utf-8');
    return JSON.parse(raw) as PaymentStore;
  } catch {
    return {
      stripeKey: null,
      stripeKeyEncrypted: false,
      customerId: null,
      subscriptionId: null,
      subscription: null,
      invoices: [],
    };
  }
}

function writePaymentStore(store: PaymentStore): void {
  ensureDataDir();
  fs.writeFileSync(PAYMENT_FILE, JSON.stringify(store, null, 2), 'utf-8');
}

function generateInvoiceId(): string {
  return `inv_${crypto.randomBytes(12).toString('hex')}`;
}

function getDecryptedStripeKey(): string | null {
  const store = readPaymentStore();
  if (!store.stripeKey) return null;
  if (store.stripeKeyEncrypted) {
    try {
      return decryptString(store.stripeKey);
    } catch {
      return null;
    }
  }
  return store.stripeKey;
}

// Lazy Stripe instance
let stripeInstance: any = null;

function getStripe(): any | null {
  const key = getDecryptedStripeKey();
  if (!key) {
    stripeInstance = null;
    return null;
  }
  try {
    // Dynamic require to avoid issues when stripe is not needed
    const Stripe = require('stripe').default || require('stripe');
    if (!stripeInstance || (stripeInstance as any)._lastKey !== key) {
      stripeInstance = new Stripe(key, { apiVersion: '2024-12-18.acacia' as any });
      (stripeInstance as any)._lastKey = key;
    }
    return stripeInstance;
  } catch (err) {
    console.error('Failed to initialize Stripe:', err);
    return null;
  }
}

function isStripeAvailable(): boolean {
  return getStripe() !== null;
}

// Temporary HTTP server for Stripe Checkout callback
function startCallbackServer(): Promise<{ port: number; waitForCallback: () => Promise<{ sessionId: string }> }> {
  return new Promise((resolve, reject) => {
    let callbackResolve: (value: { sessionId: string }) => void;
    const callbackPromise = new Promise<{ sessionId: string }>((res) => {
      callbackResolve = res;
    });

    const server = http.createServer((req, res) => {
      const url = new URL(req.url || '/', `http://localhost`);

      if (url.pathname === '/success') {
        const sessionId = url.searchParams.get('session_id') || '';
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(`
          <html>
            <body style="background:#0d1117;color:#c9d1d9;display:flex;align-items:center;justify-content:center;height:100vh;font-family:system-ui;">
              <div style="text-align:center;">
                <h1 style="color:#58a6ff;">Payment Successful!</h1>
                <p>You can close this tab and return to VidEplace.</p>
              </div>
            </body>
          </html>
        `);
        callbackResolve!({ sessionId });
        setTimeout(() => server.close(), 2000);
      } else if (url.pathname === '/cancel') {
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(`
          <html>
            <body style="background:#0d1117;color:#c9d1d9;display:flex;align-items:center;justify-content:center;height:100vh;font-family:system-ui;">
              <div style="text-align:center;">
                <h1 style="color:#f85149;">Payment Cancelled</h1>
                <p>You can close this tab and return to VidEplace.</p>
              </div>
            </body>
          </html>
        `);
        callbackResolve!({ sessionId: '' });
        setTimeout(() => server.close(), 2000);
      } else {
        res.writeHead(404);
        res.end('Not found');
      }
    });

    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      if (addr && typeof addr === 'object') {
        resolve({
          port: addr.port,
          waitForCallback: () => callbackPromise,
        });
      } else {
        reject(new Error('Failed to start callback server'));
      }
    });

    server.on('error', reject);

    // Auto-close after 10 minutes to prevent zombie servers
    setTimeout(() => {
      server.close();
      callbackResolve!({ sessionId: '' });
    }, 10 * 60 * 1000);
  });
}

export function registerPaymentHandlers(): void {
  // Set Stripe key (encrypted storage)
  ipcMain.handle('payment:setStripeKey', async (_event, key: string) => {
    try {
      if (!key || !key.startsWith('sk_')) {
        return { success: false, error: 'Invalid Stripe key format. Must start with sk_' };
      }

      const store = readPaymentStore();
      store.stripeKey = encryptString(key);
      store.stripeKeyEncrypted = true;
      writePaymentStore(store);

      // Reset cached instance
      stripeInstance = null;

      const testMode = key.startsWith('sk_test_');

      return { success: true, testMode };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // Get available plans
  ipcMain.handle('payment:getPlans', async () => {
    const stripe = getStripe();
    if (!stripe) {
      return LOCAL_PLANS;
    }

    try {
      // Fetch products and prices from Stripe
      const products = await stripe.products.list({ active: true, limit: 20 });
      const prices = await stripe.prices.list({ active: true, limit: 50 });

      const stripePlans: Plan[] = products.data.map((product: any) => {
        const productPrices = prices.data.filter(
          (p: any) => p.product === product.id && p.recurring
        );
        const monthlyPrice = productPrices.find(
          (p: any) => p.recurring?.interval === 'month'
        );

        const metadata = product.metadata || {};
        const features = metadata.features
          ? metadata.features.split(',').map((f: string) => f.trim())
          : product.description
            ? [product.description]
            : [];

        return {
          id: product.id,
          name: product.name,
          price: monthlyPrice ? monthlyPrice.unit_amount / 100 : 0,
          interval: 'month' as const,
          features,
          stripePriceId: monthlyPrice?.id,
        };
      });

      // Sort by price
      stripePlans.sort((a, b) => a.price - b.price);

      // If Stripe returned nothing useful, fallback
      if (stripePlans.length === 0) {
        return LOCAL_PLANS;
      }

      return stripePlans;
    } catch (err) {
      console.error('Failed to fetch Stripe plans, using local:', err);
      return LOCAL_PLANS;
    }
  });

  // Subscribe to a plan
  ipcMain.handle('payment:subscribe', async (_event, priceIdOrPlanId: string) => {
    const stripe = getStripe();

    // If no Stripe key, use local simulation
    if (!stripe) {
      return subscribeLocal(priceIdOrPlanId);
    }

    try {
      // Determine the price ID - either it's a Stripe price ID or a local plan ID
      let priceId = priceIdOrPlanId;

      // If it looks like a local plan ID, try to find the corresponding Stripe price
      if (!priceId.startsWith('price_')) {
        const plan = LOCAL_PLANS.find((p) => p.id === priceIdOrPlanId);
        if (plan && plan.id === 'enterprise') {
          return { success: false, error: 'Enterprise plan requires contacting sales' };
        }
        if (plan && plan.id === 'free') {
          return subscribeLocal('free');
        }
        // Try to find the price in Stripe
        try {
          const prices = await stripe.prices.list({ active: true, limit: 50 });
          const matchingPrice = prices.data.find((p: any) => {
            return p.metadata?.plan_id === priceIdOrPlanId;
          });
          if (matchingPrice) {
            priceId = matchingPrice.id;
          } else {
            // No matching Stripe price, fall back to local
            return subscribeLocal(priceIdOrPlanId);
          }
        } catch {
          return subscribeLocal(priceIdOrPlanId);
        }
      }

      // Start callback server for Checkout redirect
      const { port, waitForCallback } = await startCallbackServer();
      const successUrl = `http://127.0.0.1:${port}/success?session_id={CHECKOUT_SESSION_ID}`;
      const cancelUrl = `http://127.0.0.1:${port}/cancel`;

      // Get or create customer
      const store = readPaymentStore();
      let customerId = store.customerId;

      if (!customerId) {
        const customer = await stripe.customers.create({
          metadata: { source: 'videplace-ide' },
        });
        customerId = customer.id;
        store.customerId = customerId;
        writePaymentStore(store);
      }

      // Create Checkout Session
      const session = await stripe.checkout.sessions.create({
        customer: customerId,
        payment_method_types: ['card'],
        mode: 'subscription',
        line_items: [
          {
            price: priceId,
            quantity: 1,
          },
        ],
        success_url: successUrl,
        cancel_url: cancelUrl,
      });

      // Open in browser
      if (session.url) {
        await shell.openExternal(session.url);
      }

      // Wait for callback (non-blocking in background)
      waitForCallback().then(async (result) => {
        if (result.sessionId) {
          try {
            // Fetch the completed session
            const completedSession = await stripe.checkout.sessions.retrieve(result.sessionId);
            const updatedStore = readPaymentStore();
            updatedStore.customerId = completedSession.customer as string;
            updatedStore.subscriptionId = completedSession.subscription as string;

            // Fetch subscription details
            if (updatedStore.subscriptionId) {
              const sub = await stripe.subscriptions.retrieve(updatedStore.subscriptionId);
              updatedStore.subscription = {
                plan: priceIdOrPlanId,
                status: mapStripeStatus(sub.status),
                expiresAt: new Date(sub.current_period_end * 1000).toISOString(),
                subscribedAt: new Date(sub.created * 1000).toISOString(),
                stripeSubscriptionId: sub.id,
              };
            }

            writePaymentStore(updatedStore);

            // Also save to Supabase
            const sb = getSupabase?.();
            if (sb) {
              try {
                const { data: { user } } = await sb.auth.getUser();
                if (user) {
                  await sb.from('subscriptions').upsert({
                    user_id: user.id,
                    stripe_customer_id: updatedStore.customerId,
                    stripe_subscription_id: updatedStore.subscriptionId,
                    plan_id: priceIdOrPlanId,
                    status: 'active',
                    updated_at: new Date().toISOString(),
                  }, { onConflict: 'user_id' });
                }
              } catch { /* Supabase save failed, local storage is primary */ }
            }

            // Notify renderer
            const windows = BrowserWindow.getAllWindows();
            for (const win of windows) {
              win.webContents.send('payment:subscriptionUpdated', updatedStore.subscription);
            }
          } catch (err) {
            console.error('Failed to process checkout callback:', err);
          }
        }
      });

      return {
        success: true,
        sessionId: session.id,
        url: session.url,
        pending: true,
      };
    } catch (err: any) {
      console.error('Stripe subscribe error:', err);
      return { success: false, error: err.message };
    }
  });

  // Cancel subscription
  ipcMain.handle('payment:cancel', async () => {
    const stripe = getStripe();
    const store = readPaymentStore();

    if (!stripe || !store.subscriptionId) {
      // Local fallback
      return cancelLocal();
    }

    try {
      // Cancel at period end (Stripe best practice)
      const sub = await stripe.subscriptions.update(store.subscriptionId, {
        cancel_at_period_end: true,
      });

      store.subscription = {
        plan: store.subscription?.plan || 'unknown',
        status: 'canceled',
        expiresAt: new Date(sub.current_period_end * 1000).toISOString(),
        cancelAt: new Date(sub.current_period_end * 1000).toISOString(),
        subscribedAt: store.subscription?.subscribedAt || new Date().toISOString(),
        stripeSubscriptionId: sub.id,
      };
      writePaymentStore(store);

      // Update Supabase
      const sb = getSupabase?.();
      if (sb) {
        try {
          const { data: { user } } = await sb.auth.getUser();
          if (user) {
            await sb.from('subscriptions').update({
              status: 'canceling',
              updated_at: new Date().toISOString(),
            }).eq('user_id', user.id);
          }
        } catch { /* ignore */ }
      }

      return { success: true };
    } catch (err: any) {
      console.error('Stripe cancel error:', err);
      return { success: false, error: err.message };
    }
  });

  // Get current subscription
  ipcMain.handle('payment:getCurrentSubscription', async () => {
    // Try Supabase first
    const sb = getSupabase?.();
    if (sb) {
      try {
        const { data: { user } } = await sb.auth.getUser();
        if (user) {
          const { data } = await sb.from('subscriptions').select('*').eq('user_id', user.id).single();
          if (data) {
            return {
              plan: data.plan_id,
              status: data.status,
              expiresAt: data.current_period_end || new Date().toISOString(),
              subscribedAt: data.created_at,
              stripeSubscriptionId: data.stripe_subscription_id,
            } as Subscription;
          }
        }
      } catch { /* fallback to Stripe/local */ }
    }

    const stripe = getStripe();
    const store = readPaymentStore();

    if (stripe && store.customerId) {
      try {
        // Fetch latest subscription from Stripe
        const subscriptions = await stripe.subscriptions.list({
          customer: store.customerId,
          limit: 1,
          status: 'all',
        });

        if (subscriptions.data.length > 0) {
          const sub = subscriptions.data[0];
          const subscription: Subscription = {
            plan: sub.items.data[0]?.price?.id || store.subscription?.plan || 'unknown',
            status: mapStripeStatus(sub.status),
            expiresAt: new Date(sub.current_period_end * 1000).toISOString(),
            subscribedAt: new Date(sub.created * 1000).toISOString(),
            stripeSubscriptionId: sub.id,
          };

          if (sub.cancel_at) {
            subscription.cancelAt = new Date(sub.cancel_at * 1000).toISOString();
          }

          // Update local cache
          store.subscription = subscription;
          store.subscriptionId = sub.id;
          writePaymentStore(store);

          return subscription;
        }
      } catch (err) {
        console.error('Failed to fetch Stripe subscription, using local:', err);
      }
    }

    // Local fallback
    if (!store.subscription) return null;

    if (
      store.subscription.status === 'active' &&
      new Date(store.subscription.expiresAt) < new Date()
    ) {
      store.subscription.status = 'none';
      writePaymentStore(store);
    }

    return store.subscription;
  });

  // Get invoice history
  ipcMain.handle('payment:getInvoices', async () => {
    const stripe = getStripe();
    const store = readPaymentStore();

    if (stripe && store.customerId) {
      try {
        const invoices = await stripe.invoices.list({
          customer: store.customerId,
          limit: 20,
        });

        return invoices.data.map((inv: any) => ({
          id: inv.id,
          amount: inv.amount_paid / 100,
          date: new Date(inv.created * 1000).toISOString(),
          status: inv.status === 'paid' ? 'paid' : inv.status === 'open' ? 'pending' : 'failed',
          plan: inv.lines?.data?.[0]?.description || 'Subscription',
        }));
      } catch (err) {
        console.error('Failed to fetch Stripe invoices, using local:', err);
      }
    }

    return store.invoices;
  });

  // Create Customer Portal session
  ipcMain.handle('payment:createCustomerPortalSession', async () => {
    const stripe = getStripe();
    const store = readPaymentStore();

    if (!stripe) {
      return { success: false, error: 'Stripe is not configured. Set a Stripe key first.' };
    }

    if (!store.customerId) {
      return { success: false, error: 'No customer account found. Subscribe to a plan first.' };
    }

    try {
      const session = await stripe.billingPortal.sessions.create({
        customer: store.customerId,
        return_url: 'https://videplace.dev',
      });

      if (session.url) {
        await shell.openExternal(session.url);
      }

      return { success: true, url: session.url };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // Sync subscription status from Stripe
  ipcMain.handle('payment:syncSubscriptionStatus', async () => {
    const stripe = getStripe();
    const store = readPaymentStore();

    if (!stripe || !store.customerId) {
      return { success: false, error: 'Stripe not configured or no customer ID' };
    }

    try {
      const subscriptions = await stripe.subscriptions.list({
        customer: store.customerId,
        limit: 1,
        status: 'all',
      });

      if (subscriptions.data.length > 0) {
        const sub = subscriptions.data[0];
        store.subscription = {
          plan: sub.items.data[0]?.price?.id || 'unknown',
          status: mapStripeStatus(sub.status),
          expiresAt: new Date(sub.current_period_end * 1000).toISOString(),
          subscribedAt: new Date(sub.created * 1000).toISOString(),
          stripeSubscriptionId: sub.id,
        };
        if (sub.cancel_at) {
          store.subscription.cancelAt = new Date(sub.cancel_at * 1000).toISOString();
        }
        store.subscriptionId = sub.id;
        writePaymentStore(store);

        return { success: true, subscription: store.subscription };
      }

      return { success: true, subscription: null };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });
}

// Map Stripe subscription status to our status type
function mapStripeStatus(
  stripeStatus: string
): 'active' | 'canceled' | 'past_due' | 'trialing' | 'none' {
  switch (stripeStatus) {
    case 'active':
      return 'active';
    case 'canceled':
      return 'canceled';
    case 'past_due':
      return 'past_due';
    case 'trialing':
      return 'trialing';
    case 'incomplete':
    case 'incomplete_expired':
    case 'unpaid':
    default:
      return 'none';
  }
}

// Local simulation fallback
function subscribeLocal(planId: string): { success: boolean; error?: string; subscription?: Subscription } {
  try {
    const plan = LOCAL_PLANS.find((p) => p.id === planId);
    if (!plan) {
      return { success: false, error: 'Invalid plan' };
    }

    if (planId === 'enterprise') {
      return { success: false, error: 'Enterprise plan requires contacting sales' };
    }

    const store = readPaymentStore();
    const now = new Date();
    const expiresAt = new Date(now);
    expiresAt.setMonth(expiresAt.getMonth() + 1);

    const subscription: Subscription = {
      plan: planId,
      status: 'active',
      expiresAt: expiresAt.toISOString(),
      subscribedAt: now.toISOString(),
    };

    store.subscription = subscription;

    if (plan.price > 0) {
      const invoice: Invoice = {
        id: generateInvoiceId(),
        amount: plan.price,
        date: now.toISOString(),
        status: 'paid',
        plan: planId,
      };
      store.invoices.unshift(invoice);
    }

    writePaymentStore(store);

    return { success: true, subscription };
  } catch (err: any) {
    return { success: false, error: err.message };
  }
}

function cancelLocal(): { success: boolean; error?: string } {
  try {
    const store = readPaymentStore();

    if (!store.subscription || store.subscription.plan === 'free') {
      return { success: false, error: 'No active paid subscription to cancel' };
    }

    store.subscription.status = 'canceled';
    store.subscription.cancelAt = store.subscription.expiresAt;
    writePaymentStore(store);

    return { success: true };
  } catch (err: any) {
    return { success: false, error: err.message };
  }
}
