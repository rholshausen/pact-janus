// The web app's own code — what the consumer test exercises. It fetches an order from
// order-service and turns it into the line the order page shows.

export interface Order {
  id: string;
  status: string;
  items: { sku: string; quantity: number }[];
}

export class OrderClient {
  constructor(readonly baseUrl: string) {}

  async getOrder(id: string): Promise<Order> {
    const response = await fetch(`${this.baseUrl}/orders/${id}`, { headers: { Accept: "application/json" } });
    if (!response.ok) {
      throw new Error(`GET /orders/${id} answered ${response.status}`);
    }
    return (await response.json()) as Order;
  }
}

const STATUS = new Map([
  ["PENDING", "being prepared"],
  ["SHIPPED", "on its way"],
]);

/** "Order 42 (on its way): 1 × sku-0 and 1 more" */
export function summarise(order: Order): string {
  const status = STATUS.get(order.status);
  if (status === undefined) {
    throw new Error(`unknown order status '${order.status}'`);
  }
  const [first, ...rest] = order.items;
  const more = rest.length > 0 ? ` and ${rest.length} more` : "";
  return `Order ${order.id} (${status}): ${first.quantity} × ${first.sku}${more}`;
}
