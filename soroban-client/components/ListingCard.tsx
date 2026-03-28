'use client';

import { useState } from 'react';
import { useWallet } from '@/contexts/WalletContext';

interface ListingCardProps {
  listing: any;
  onPurchase: () => void;
}

export const ListingCard = ({ listing, onPurchase }: ListingCardProps) => {
  const { address, isConnected, connect } = useWallet();
  const [purchasing, setPurchasing] = useState(false);
  const [error, setError] = useState('');

  const handlePurchase = async () => {
    if (!isConnected) {
      await connect();
      return;
    }

    try {
      setPurchasing(true);
      setError('');
      
      // Call marketplace contract purchase function
      const response = await fetch('/api/marketplace/purchase', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          listingId: listing.id,
          buyerAddress: address,
        }),
      });

      const data = await response.json();
      
      if (data.success) {
        alert('Ticket purchased successfully!');
        onPurchase();
      } else {
        setError(data.error || 'Purchase failed');
      }
    } catch (error) {
      console.error('Purchase failed:', error);
      setError('Failed to purchase ticket. Please try again.');
    } finally {
      setPurchasing(false);
    }
  };

  return (
    <div className="bg-gray-800 rounded-lg overflow-hidden shadow-lg hover:shadow-xl transition">
      <div className="p-6">
        <div className="flex justify-between items-start mb-4">
          <h3 className="text-xl font-semibold text-white">
            Ticket #{listing.tokenId}
          </h3>
          <span className="bg-blue-600 text-white px-3 py-1 rounded-full text-sm">
            {listing.price} XLM
          </span>
        </div>
        
        <div className="space-y-2 text-gray-300 text-sm mb-6">
          <p>Seller: {listing.seller.slice(0, 6)}...{listing.seller.slice(-4)}</p>
          <p>Listed: {new Date(listing.createdAt * 1000).toLocaleDateString()}</p>
          <p>Contract: {listing.ticketContract.slice(0, 6)}...{listing.ticketContract.slice(-4)}</p>
        </div>
        
        {error && (
          <div className="mb-4 p-3 bg-red-900/50 border border-red-700 rounded text-red-200 text-sm">
            {error}
          </div>
        )}
        
        <button
          onClick={handlePurchase}
          disabled={purchasing || listing.seller === address}
          className={`w-full py-2 rounded-lg transition ${
            listing.seller === address
              ? 'bg-gray-600 cursor-not-allowed'
              : 'bg-green-600 hover:bg-green-700'
          } text-white font-medium`}
        >
          {purchasing
            ? 'Processing...'
            : listing.seller === address
            ? 'Your Listing'
            : isConnected
            ? 'Purchase Ticket'
            : 'Connect Wallet to Purchase'}
        </button>
      </div>
    </div>
  );
};