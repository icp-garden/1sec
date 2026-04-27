// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";

/**
 * @dev Implements an ERC20 token that wraps an ICP token using the
 * lock-mint/burn-unlock approach.
 */
contract Token is ERC20, Ownable, ERC20Permit {
    event Burn1(address from, uint256 amount, bytes32 data1);
    event Burn2(address from, uint256 amount, bytes32 data1, bytes32 data2);
    event Burn3(address from, uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3);
    event Burn4(address from, uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3, bytes32 data4);

    uint8 _decimals;
    uint256 public minAmount;

    /**
     * @dev Initializes the ERC20 token.
     *
     * @param tokenName The name the token.
     * @param decimals_ The decimals of the token.
     * @param minAmount_ The minimum withdraw amount.
     */
    constructor(string memory tokenName, uint8 decimals_, uint256 minAmount_)
        ERC20(tokenName, tokenName)
        Ownable(msg.sender)
        ERC20Permit(tokenName)
    {
        _decimals = decimals_;
        minAmount = minAmount_;
    }

    /**
     * @dev Burns the given amount of tokens and emits a `Burn1` event in
     * order to notify the ICP contract to unlock the corresponding amount of
     * tokens to the given recipient.
     *
     * @param amount The amount to burn.
     * @param data1 The encoded ICP recipient.
     */
    function burn1(uint256 amount, bytes32 data1) public {
        require(amount >= minAmount, "Amount is too low");
        _burn(msg.sender, amount);
        emit Burn1(msg.sender, amount, data1);
    }

    /**
     * @dev Burns the given amount of tokens and emits a `Burn2` event in
     * order to notify the ICP contract to unlock the corresponding amount of
     * tokens to the given recipient.
     *
     * @param amount The amount to burn.
     * @param data1 A part of the encoded ICP recipient.
     * @param data2 A part of the encoded ICP recipient.
     */
    function burn2(uint256 amount, bytes32 data1, bytes32 data2) public {
        require(amount >= minAmount, "Amount is too low");
        _burn(msg.sender, amount);
        emit Burn2(msg.sender, amount, data1, data2);
    }

    /**
     * @dev Burns the given amount of tokens and emits a `Burn3` event in
     * order to notify the ICP contract to unlock the corresponding amount of
     * tokens to the given recipient.
     *
     * @param amount The amount to burn.
     * @param data1 A part of the encoded ICP recipient.
     * @param data2 A part of the encoded ICP recipient.
     * @param data3 A part of the encoded ICP recipient.
     */
    function burn3(uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3) public {
        require(amount >= minAmount, "Amount is too low");
        _burn(msg.sender, amount);
        emit Burn3(msg.sender, amount, data1, data2, data3);
    }

    /**
     * @dev Burns the given amount of tokens and emits a `Burn4` event in
     * order to notify the ICP contract to unlock the corresponding amount of
     * tokens to the given recipient.
     *
     * @param amount The amount to burn.
     * @param data1 A part of the encoded ICP recipient.
     * @param data2 A part of the encoded ICP recipient.
     * @param data3 A part of the encoded ICP recipient.
     * @param data4 A part of the encoded ICP recipient.
     */
    function burn4(uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3, bytes32 data4) public {
        require(amount >= minAmount, "Amount is too low");
        _burn(msg.sender, amount);
        emit Burn4(msg.sender, amount, data1, data2, data3, data4);
    }

    /**
     * @dev Mint tokens to the given address and forwards any eth attached.
     */
    function mint(address payable to, uint256 amount) public payable onlyOwner {
        // No need to enforce the minimum amount because this function can be
        // called only by the owner.
        _mint(to, amount);
        if (msg.value > 0) {
            // We want to continue progress if to is not an EOA.
            // Safe to ignore success as ETH amounts are minimal and this is a convenience feature.
            (bool success,) = to.call{value: msg.value, gas: 2300}("");
            success;
        }
    }

    /**
     * @dev Mints tokens in batch from the owner's address to multiple recipients.
     *      Only the contract owner can call this function.
     *      If ETH is attached, msg.value will be dispatched following ethAmounts array.
     * @param recipients Array of addresses to receive the tokens.
     * @param amounts Array of token amounts to send to each recipient.
     *        The order of recipients and amounts must match (i.e., recipients[i] gets amounts[i]).
     * @param ethAmounts Array of eth amounts to send to each recipient.
     *        The order of recipients and ethAmounts must match (i.e., recipients[i] gets ethAmounts[i]).
     *        Condition: msg.value MUST be greater than or equal to the total of ethAmounts.
     */
    function batchMint(address payable[] calldata recipients, uint256[] calldata amounts, uint256[] calldata ethAmounts)
        public
        payable
        onlyOwner
    {
        require(recipients.length == amounts.length, "Mismatched array lengths");
        if (msg.value == 0) {
            for (uint256 i = 0; i < recipients.length; i++) {
                _mint(recipients[i], amounts[i]);
            }
        } else {
            require(recipients.length == ethAmounts.length, "Mismatched array lengths");

            for (uint256 i = 0; i < recipients.length; i++) {
                _mint(recipients[i], amounts[i]);
                if (ethAmounts[i] > 0) {
                    // We want to continue progress if recipients[i] is not an EOA.
                    // Safe to ignore success as ETH amounts are minimal and this is a convenience feature.
                    (bool success,) = recipients[i].call{value: ethAmounts[i], gas: 2300}("");
                    success;
                }
            }
        }
    }

    /**
     * @dev Withdraw eth from the contract.
     */
    function withdrawEth(uint256 amount, address payable to) public onlyOwner {
        require(amount <= address(this).balance, "Balance too low");
        to.transfer(amount);
    }

    /**
     * @dev Updates the owner of the erc20 contract.
     */
    function updateOwner(address _owner) public onlyOwner {
        _transferOwnership(_owner);
    }

    /**
     * @dev Updates the minimum withdraw amount.
     */
    function updateMinAmount(uint256 _minAmount) public onlyOwner {
        minAmount = _minAmount;
    }

    /**
     * @dev Returns the number of decimals used for token representation.
     */
    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }
}
